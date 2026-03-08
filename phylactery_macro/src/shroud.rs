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

fn combinations<T>(mut items: &[T]) -> Vec<Vec<&T>> {
    let n = items.len();
    let mut groups = Vec::with_capacity(1 << n);
    groups.push(Vec::new());
    while let Some((head, tail)) = items.split_first() {
        groups.push(vec![head]);
        for size in 1..=tail.len() {
            for subset in choose(tail, size) {
                let mut group = Vec::with_capacity(size + 1);
                group.push(head);
                group.extend(subset);
                groups.push(group);
            }
        }
        items = tail;
    }
    groups
}

fn choose<T>(items: &[T], k: usize) -> Vec<Vec<&T>> {
    if k == 0 {
        return vec![vec![]];
    }
    let Some((head, tail)) = items.split_first() else {
        return vec![];
    };
    let mut result = Vec::new();
    for subset in choose(tail, k - 1) {
        let mut group = Vec::with_capacity(k);
        group.push(head);
        group.extend(subset);
        result.push(group);
    }
    result.extend(choose(tail, k));
    result
}

#[test]
fn produces_all_combinations() {
    assert_eq!(combinations::<usize>(&[]), vec![vec![&0usize; 0]]);
    assert_eq!(combinations(&['a']), vec![vec![], vec![&'a']]);
    assert_eq!(
        combinations(&['a', 'b']),
        vec![vec![], vec![&'a'], vec![&'a', &'b'], vec![&'b']]
    );
    assert_eq!(
        combinations(&['a', 'b', 'c']),
        vec![
            vec![],
            vec![&'a'],
            vec![&'a', &'b'],
            vec![&'a', &'c'],
            vec![&'a', &'b', &'c'],
            vec![&'b'],
            vec![&'b', &'c'],
            vec![&'c']
        ]
    );
    assert_eq!(
        combinations(&['a', 'b', 'c', 'd']),
        vec![
            vec![],
            vec![&'a'],
            vec![&'a', &'b'],
            vec![&'a', &'c'],
            vec![&'a', &'d'],
            vec![&'a', &'b', &'c'],
            vec![&'a', &'b', &'d'],
            vec![&'a', &'c', &'d'],
            vec![&'a', &'b', &'c', &'d'],
            vec![&'b'],
            vec![&'b', &'c'],
            vec![&'b', &'d'],
            vec![&'b', &'c', &'d'],
            vec![&'c'],
            vec![&'c', &'d'],
            vec![&'d']
        ]
    );
}

/// Regression test for Issue 03: `combinations()` must produce all 2^N subsets.
///
/// The original algorithm only generates contiguous sub-slices, missing
/// non-contiguous combinations for N ≥ 4.  E.g. for N=4 it produces 15
/// subsets instead of 16, silently omitting `[a, b, d]`.
///
/// This test currently FAILS with the buggy implementation (15 ≠ 16).
/// It will PASS after the fix replaces the algorithm with a correct power-set.
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
    // Specifically, the non-contiguous subset [a, b, d] (skipping c) must be present.
    let char_a = &'a';
    let char_b = &'b';
    let char_d = &'d';
    assert!(
        result.contains(&vec![char_a, char_b, char_d]),
        "combinations() is missing [a, b, d] for N=4 (Issue 03)"
    );
}
