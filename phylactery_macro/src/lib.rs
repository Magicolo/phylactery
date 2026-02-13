#![forbid(unsafe_code)]

mod shroud;
use crate::shroud::Shroud;
use quote::{quote, quote_spanned};
use syn::{
    parse_macro_input, ConstParam, GenericParam, Generics, ItemTrait, LifetimeParam, TraitItem,
    TraitItemType, TypeParam,
};

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
                    #[automatically_derived]
                    impl<'__life_in__, '__life_out__: '__life_in__, #(#parameters,)*> ::phylactery::Shroud<dyn #ident<#(#parameter_names,)* #(#assigns,)*> #(+ #paths)* + '__life_in__> for dyn #ident<#(#parameter_names,)* #(#assigns,)*> #(+ #paths)* + '__life_out__ #where_clause {
                        #[inline(always)]
                        fn shroud(from: ::core::ptr::NonNull<dyn #ident<#(#parameter_names,)* #(#assigns,)*> #(+ #paths)* + '__life_in__>) -> ::core::ptr::NonNull<Self> {
                            unsafe {
                                ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
                                    *mut (dyn #ident<#(#parameter_names,)* #(#assigns,)*> #(+ #paths)* + '__life_in__),
                                    *mut (dyn #ident<#(#parameter_names,)* #(#assigns,)*> #(+ #paths)* + '__life_out__)
                                >(from.as_ptr() as _))
                            }
                        }
                    }
                )
            } else {
                quote_spanned!(span =>
                    #[automatically_derived]
                    impl<'__life__, #(#parameters,)* __TConcrete__: #ident<#(#parameter_names,)*> #(+ #paths)*> ::phylactery::Shroud<__TConcrete__> for dyn #ident<#(#parameter_names,)* #(#associates = __TConcrete__::#associates,)*> #(+ #paths)* + '__life__ #where_clause {
                        #[inline(always)]
                        fn shroud(from: ::core::ptr::NonNull<__TConcrete__>) -> ::core::ptr::NonNull<Self> {
                            unsafe {
                                ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
                                    *mut (dyn #ident<#(#parameter_names,)* #(#associates = __TConcrete__::#associates,)*> #(+ #paths)*),
                                    *mut Self
                                >(from.as_ptr() as _))
                            }
                        }
                    }
                )
            }
        });
    quote! { #item #(#implementations)* }.into()
}
