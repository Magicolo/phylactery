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
    pub combine: bool,
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
                }) => shroud.combine = true,
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
            combine: false,
            paths: Vec::new(),
            assigns: Vec::new(),
        }
    }

    pub fn paths(&self) -> Vec<Vec<&ExprPath>> {
        if self.combine {
            combinations(&self.paths)
        } else {
            vec![self.paths.iter().collect()]
        }
    }
}

impl Shroud {
    pub fn try_from(value: &Attribute) -> Result<Self, Error> {
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

fn combinations<T>(items: &[T]) -> Vec<Vec<&T>> {
    let count = 1usize << items.len();
    let mut groups = Vec::with_capacity(count);
    for mask in 0..count {
        let group: Vec<&T> = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if mask & (1 << i) != 0 {
                    Some(item)
                } else {
                    None
                }
            })
            .collect();
        groups.push(group);
    }
    groups
}

#[test]
fn produces_all_combinations() {
    for count in 0..=10usize {
        let items: Vec<usize> = (0..count).collect();
        let result = combinations(&items);
        assert_eq!(result.len(), 1 << count, "wrong count for n={count}");
        for mask in 0..(1usize << count) {
            let expected: Vec<&usize> = items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    if mask & (1 << index) != 0 {
                        Some(item)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(
                result.contains(&expected),
                "missing subset {mask:b} for n={count}"
            );
        }
    }
}

/// Regression test for Issue 03: `combinations()` must produce all 2^N subsets.
///
/// The original algorithm only generated contiguous sub-slices, missing
/// non-contiguous combinations for N ≥ 4.  E.g. for N=4 it produced 15
/// subsets instead of 16, silently omitting `[a, b, d]`.
#[test]
fn combinations_produces_correct_count_for_n4() {
    // For N=4, the power set has exactly 2^4 = 16 elements.
    let result = combinations(&['a', 'b', 'c', 'd']);
    assert_eq!(
        result.len(),
        16,
        "combinations() produced {} subsets for N=4, expected 16 (Issue 03)",
        result.len()
    );
    // Specifically, the non-contiguous subset [a, b, d] (skipping c) must be
    // present.
    let char_a = &'a';
    let char_b = &'b';
    let char_d = &'d';
    assert!(
        result.contains(&vec![char_a, char_b, char_d]),
        "combinations() is missing [a, b, d] for N=4 (Issue 03)"
    );
}
