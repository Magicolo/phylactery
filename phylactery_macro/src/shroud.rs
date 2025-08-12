use core::mem::replace;
use quote::ToTokens;
use syn::{
    __private::Span,
    Attribute, Error, Expr, ExprAssign, ExprPath, ExprRange, Ident, Meta, Path, PathSegment,
    RangeLimits,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

#[derive(Clone)]
pub struct Shroud {
    pub span: Span,
    pub dynamic: bool,
    pub default: bool,
    pub paths: Vec<ExprPath>,
    pub assigns: Vec<ExprAssign>,
}

impl Parse for Shroud {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        const DYNAMIC: [&str; 3] = ["dyn", "self", "Self"];
        let mut shroud = Shroud::new(input.span());
        for expression in Punctuated::<Expr, Comma>::parse_terminated(input)? {
            match expression {
                Expr::Path(ExprPath { path, .. })
                    if DYNAMIC.iter().any(|name| path.is_ident(name)) =>
                {
                    shroud.dynamic = true;
                }
                Expr::Range(ExprRange {
                    start: None,
                    end: None,
                    limits: RangeLimits::HalfOpen(_),
                    ..
                }) => shroud.default = true,
                Expr::Path(path) => shroud.paths.push(path),
                Expr::Assign(assign) => shroud.assigns.push(assign),
                expression => {
                    return Err(error(expression, |key| {
                        format!("invalid expression '{key}'")
                    }));
                }
            }
        }
        Ok(shroud)
    }
}

impl Shroud {
    pub fn new(span: Span) -> Self {
        Self {
            span,
            dynamic: false,
            default: false,
            paths: Vec::new(),
            assigns: Vec::new(),
        }
    }

    pub fn dynamic(self, dynamic: bool) -> Self {
        Self { dynamic, ..self }
    }

    pub fn path(mut self, path: ExprPath) -> Self {
        self.paths.push(path);
        self
    }

    pub fn paths(mut self, paths: impl IntoIterator<Item = ExprPath>) -> Self {
        self.paths.extend(paths);
        self
    }
}

impl TryFrom<&Attribute> for Shroud {
    type Error = Error;

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        const PATHS: [&[&str]; 2] = [&["phylactery", "shroud"], &["shroud"]];

        let path = value.path();
        if PATHS.into_iter().any(|legal| idents(path).eq(legal)) {
            if matches!(value.meta, Meta::Path(_)) {
                Ok(Shroud::new(value.span()))
            } else {
                value.meta.require_list()?.parse_args()
            }
        } else {
            Err(error(path, |path| {
                let paths = PATHS.into_iter().map(|path| join("::", path));
                format!(
                    "invalid attribute path '{path}'\nmust be one of [{}]",
                    join(", ", paths)
                )
            }))
        }
    }
}

fn string<T: ToTokens>(tokens: &T) -> String {
    tokens.to_token_stream().to_string()
}

fn error<T: ToTokens>(tokens: T, format: impl FnOnce(String) -> String) -> Error {
    let message = format(string(&tokens));
    Error::new_spanned(tokens, message)
}

fn idents(path: &Path) -> impl Iterator<Item = &Ident> {
    path.segments.iter().map(|PathSegment { ident, .. }| ident)
}

fn join<S: AsRef<str>, I: AsRef<str>>(separator: S, items: impl IntoIterator<Item = I>) -> String {
    let mut buffer = String::new();
    let mut join = false;
    let separator = separator.as_ref();
    for item in items {
        if replace(&mut join, true) {
            buffer.push_str(separator);
        }
        buffer.push_str(item.as_ref());
    }
    buffer
}
