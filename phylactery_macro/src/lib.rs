#![forbid(unsafe_code)]

mod shroud;
use crate::shroud::Shroud;
use quote::{quote, quote_spanned};
use syn::{
    ConstParam, GenericParam, Generics, ItemTrait, LifetimeParam, TraitItem, TraitItemType,
    TypeParam, parse_macro_input,
};

/// A convenience macro to implement the [`Shroud<T>`] trait for a given trait.
/// The macro is applied to a trait directly because it will derive blanket
/// implementations of [`Shroud<T>`] for all `T: Trait`. It can also handle
/// implementing [`Shroud<T>`] for all combinations of marker traits [`Send`],
/// [`Sync`] and [`Unpin`] (ex: `dyn Trait + Send `, `dyn Trait + Sync`, `dyn
/// Trait + Send + Unpin`, etc.).
///
/// # Usage
///
/// ```
/// use phylactery::shroud;
///
/// // Generates `impl<T: Simple> Shroud<T> for dyn Simple`.
/// #[shroud]
/// pub trait Simple {}
///
/// // The `..` will generate implementations for all combinations of the specified
/// // traits. In this case `dyn Combine`, `dyn Combine + Send`, `dyn Combine + Sync + Unpin`,
/// // `dyn Combine + Send + Sync + Unpin`, etc. Be wary that the number of implementations
/// // can be very large if used with many traits.
/// #[shroud(Send, Sync, Unpin, ..)]
/// pub trait Combine {}
///
/// // Instead of `..`, the combinations can be specified manually by adding multiple `#[shroud]`.
/// #[shroud]
/// #[shroud(Send)]
/// #[shroud(Sync)]
/// #[shroud(Send, Sync)]
/// // `Self` means that the implementation will be for
/// // `Shroud<dyn Trait> for dyn Trait` rather than a blanket implementation.
/// // In that case, associated types must be specified explicitly (here with `A = usize`).
/// #[shroud(Self, A = usize)]
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
    let implementations = shrouds
        .iter()
        .flat_map(|shroud| shroud
            .paths()
            .into_iter()
            .map(|paths| (shroud.span, shroud.dynamic, shroud.assigns.clone(), paths)))
        .map(|(span, dynamic, assigns, paths)| {
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
        });
    quote! { #item #(#implementations)* }.into()
}
