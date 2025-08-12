#![forbid(unsafe_code)]

mod shroud;
use crate::shroud::Shroud;
use quote::{quote, quote_spanned};
use syn::{
    ConstParam, ExprPath, GenericParam, Generics, ItemTrait, LifetimeParam, TraitItem,
    TraitItemType, TypeParam, parse_macro_input, parse_quote_spanned,
};

/// A convenience macro to implement the [`Shroud<T>`] trait for a given trait.
/// It can also handle implementing [`Shroud<T>`] for all combinations of
/// [`Send`], [`Sync`] and [`Unpin`] (ex: `dyn Trait + Send `,
/// `dyn Trait + Sync`, `dyn Trait + Send + Unpin`, etc.).
///
/// # Usage
///
/// ```
/// use phylactery::shroud;
///
/// #[shroud]
/// pub trait Simple {}
///
/// // Generates all common implementations of [`Shroud<T>`] including all
/// // the combinations of `dyn Trait` with `Send`, `Sync` and `Unpin`.
/// #[shroud(..)]
/// pub trait Default {}
///
/// #[shroud]
/// #[shroud(Send)]
/// #[shroud(Sync)]
/// #[shroud(Send, Sync)]
/// #[shroud(A = usize, Self)]
/// pub trait Complex<'a, T: Debug, U: FromStr + 'a, const N: usize>: Simple
/// where
///     for<'b> &'b T: Display,
/// {
///     type A;
/// }
/// ```
#[proc_macro_attribute]
pub fn shroud(
    attribute: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let shroud: Shroud = parse_macro_input!(attribute);
    let mut item: ItemTrait = parse_macro_input!(item);
    let mut shrouds = vec![shroud];
    item.attrs.retain(|attribute| {
        if let Ok(shroud) = Shroud::try_from(attribute) {
            shrouds.push(shroud);
            false
        } else {
            true
        }
    });
    let item @ ItemTrait {
        ident,
        generics: Generics {
            params,
            where_clause,
            ..
        },
        items,
        ..
    } = &item;
    let mut default = None;
    shrouds.retain(|shroud| {
        if shroud.default {
            default = Some(shroud.clone());
            false
        } else {
            true
        }
    });
    if let Some(shroud) = default {
        let send: ExprPath = parse_quote_spanned!(shroud.span => Send);
        let sync: ExprPath = parse_quote_spanned!(shroud.span => Sync);
        let unpin: ExprPath = parse_quote_spanned!(shroud.span => Unpin);
        for dynamic in [true, false] {
            shrouds.push(shroud.clone().dynamic(dynamic));
            shrouds.push(shroud.clone().dynamic(dynamic).path(send.clone()));
            shrouds.push(shroud.clone().dynamic(dynamic).path(sync.clone()));
            shrouds.push(shroud.clone().dynamic(dynamic).path(unpin.clone()));
            shrouds.push(
                shroud
                    .clone()
                    .dynamic(dynamic)
                    .paths([send.clone(), sync.clone()]),
            );
            shrouds.push(
                shroud
                    .clone()
                    .dynamic(dynamic)
                    .paths([send.clone(), unpin.clone()]),
            );
            shrouds.push(
                shroud
                    .clone()
                    .dynamic(dynamic)
                    .paths([sync.clone(), unpin.clone()]),
            );
            shrouds.push(shroud.clone().dynamic(dynamic).paths([
                send.clone(),
                sync.clone(),
                unpin.clone(),
            ]));
        }
    }

    let parameters = params.iter().collect::<Vec<_>>();
    let parameter_names = parameters
        .iter()
        .map(|parameter| match parameter {
            GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => quote!(#lifetime),
            GenericParam::Type(TypeParam { ident, .. }) => quote!(#ident),
            GenericParam::Const(ConstParam { ident, .. }) => quote!(#ident),
        })
        .collect::<Vec<_>>();
    let associates = items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Type(TraitItemType {
                ident,
                generics: Generics { params, .. },
                ..
            }) if params.is_empty() => Some(ident),
            _ => None,
        })
        .collect::<Vec<_>>();
    let implementations = shrouds.into_iter().map(|Shroud { span, dynamic, paths, assigns, .. }|
        if dynamic {
            quote_spanned!(span =>
                impl<'__life_in__, '__life_out__: '__life_in__, #(#parameters,)*> ::phylactery::shroud::Shroud<dyn #ident<#(#parameter_names,)* #(#assigns,)*> #(+ #paths)* + '__life_in__> for dyn #ident<#(#parameter_names,)* #(#assigns,)*> #(+ #paths)* + '__life_out__ #where_clause {
                    #[inline(always)]
                    fn shroud(from: &(dyn #ident<#(#parameter_names,)* #(#assigns,)*> #(+ #paths)* + '__life_in__)) -> ::core::ptr::NonNull<Self> {
                        unsafe { ::core::ptr::NonNull::new_unchecked(from as *const _ as *mut _) }
                    }
                }
            )
        } else {
            quote_spanned!(span =>
                impl<'__life__, #(#parameters,)* __TConcrete__: #ident<#(#parameter_names,)*> #(+ #paths)*> ::phylactery::shroud::Shroud<__TConcrete__> for dyn #ident<#(#parameter_names,)* #(#associates = __TConcrete__::#associates,)*> #(+ #paths)* + '__life__ #where_clause {
                    #[inline(always)]
                    fn shroud(from: &__TConcrete__) -> ::core::ptr::NonNull<Self> {
                        unsafe { ::core::ptr::NonNull::new_unchecked(from as *const __TConcrete__ as *const Self as *mut Self) }
                    }
                }
            )
        }
    );
    quote! { #item #(#implementations)* }.into()
}
