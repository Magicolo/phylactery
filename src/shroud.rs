pub trait Shroud<T: ?Sized> {
    fn shroud(from: &T) -> *const Self;
}

#[macro_export]
macro_rules! shroud {
    ($type: ident) => {
        shroud!(@TRAIT { type: $type, generics: (), traits: () });
    };
    ($type: ident +) => {
        shroud!(@TRAIT { type: $type, generics: (), traits: ((Send), (Sync), (Unpin), (Send, Sync), (Send, Unpin), (Sync, Unpin), (Send, Sync, Unpin)) });
    };
    ($type: ident $(+ $trait: ident)+) => {
        shroud!(@TRAIT { type: $type, generics: (), traits: (($($trait),*)) });
    };
    ($type: ident<$($generic: ident),* $(,)?>) => {
        shroud!(@TRAIT { type: $type, generics: ($($generic),*), traits: () });
    };
    ($type: ident<$($generic: ident),* $(,)?> +) => {
        shroud!(@TRAIT { type: $type, generics: ($($generic),*), traits: ((Send), (Sync), (Unpin), (Send, Sync), (Send, Unpin), (Sync, Unpin), (Send, Sync, Unpin)) });
    };
    ($type: ident<$($generic: ident),* $(,)?> $(+ $trait: ident)+) => {
        shroud!(@TRAIT { type: $type, generics: ($($generic),*), traits: (($($trait),*)) });
    };
    (@TRAIT { type: $type: ident, generics: $generics: tt, traits: () $(,)? }) => {
        shroud!(@IMPLEMENT { type: $type, generics: $generics, traits: () });
    };
    (@TRAIT { type: $type: ident, generics: $generics: tt, traits: ($trait: tt $(, $traits: tt)*) $(,)? }) => {
        shroud!(@TRAIT { type: $type, generics: $generics, traits: ($($traits),*) });
        shroud!(@IMPLEMENT { type: $type, generics: $generics, traits: $trait });
    };
    (@IMPLEMENT { type: $type: ident, generics: ($($generic: ident),*), traits: ($($trait: path),*) $(,)? }) => {
        impl<$($generic,)*> $crate::shroud::Shroud<dyn $type<$($generic),*> $(+ $trait)*> for dyn $type<$($generic),*> $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: &(dyn $type<$($generic),*> $(+ $trait)*)) -> *const (dyn $type<$($generic),*> $(+ $trait)*) {
                from as *const _ as *const _
            }
        }

        impl<$($generic,)* TConcrete: $type<$($generic),*> $(+ $trait)*> $crate::shroud::Shroud<TConcrete> for dyn $type<$($generic),*> $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: &TConcrete) -> *const (dyn $type<$($generic),*> $(+ $trait)*) {
                from as *const _ as *const _
            }
        }
    };
}

macro_rules! shroud_fn {
    ($function: ident($(,)?) -> $return: ident) => {
        shroud_fn!(@TRAIT {
            function: $function,
            parameters: (),
            return: $return,
            traits: ((Send), (Sync), (Unpin), (Send, Sync), (Send, Unpin), (Sync, Unpin), (Send, Sync, Unpin)),
        });
    };
    ($function: ident($parameter: ident $(, $parameters: ident)* $(,)?) -> $return: ident) => {
        shroud_fn!($function($($parameters),*) -> $return);
        shroud_fn!(@TRAIT {
            function: $function,
            parameters: ($parameter $(, $parameters)*),
            return: $return,
            traits: ((Send), (Sync), (Unpin), (Send, Sync), (Send, Unpin), (Sync, Unpin), (Send, Sync, Unpin)),
        });
    };
    (@TRAIT { function: $function: ident, parameters: $parameters: tt, return: $return: ident, traits: () $(,)? }) => {
        shroud_fn!(@IMPLEMENT { function: $function, parameters: $parameters, return: $return, traits: () });
    };
    (@TRAIT { function: $function: ident, parameters: $parameters: tt, return: $return: ident, traits: ($trait: tt $(, $traits: tt)*) $(,)? }) => {
        shroud_fn!(@TRAIT { function: $function, parameters: $parameters, return: $return, traits: ($($traits),*) });
        shroud_fn!(@IMPLEMENT { function: $function, parameters: $parameters, return: $return, traits: $trait });
    };
    (@IMPLEMENT { function: $function: ident, parameters: ($($parameter: ident),*), return: $return: ident, traits: ($($trait: path),*) $(,)? }) => {
        impl<$($parameter,)* $return> $crate::shroud::Shroud<dyn $function($($parameter),*) -> $return $(+ $trait)*> for dyn $function($($parameter),*) -> $return $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: &(dyn $function($($parameter),*) -> $return $(+ $trait)*)) -> *const (dyn $function($($parameter),*) -> $return $(+ $trait)*) {
                from as *const _ as *const _
            }
        }

        impl<$($parameter,)* $return, TConcrete: $function($($parameter),*) -> $return $(+ $trait)*> $crate::shroud::Shroud<TConcrete> for dyn $function($($parameter),*) -> $return $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: &TConcrete) -> *const (dyn $function($($parameter),*) -> $return $(+ $trait)*) {
                from as *const _ as *const _
            }
        }
    };
}

shroud_fn!(Fn(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
shroud_fn!(FnMut(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
shroud_fn!(FnOnce(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
