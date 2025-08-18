pub trait Pointer {
    type Target: ?Sized;
    fn pointer(&self) -> *const Self::Target;
}

#[cfg(feature = "std")]
impl<T: ?Sized> Pointer for std::sync::Arc<T> {
    type Target = T;

    fn pointer(&self) -> *const Self::Target {
        Self::as_ref(self)
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized> Pointer for std::rc::Rc<T> {
    type Target = T;

    fn pointer(&self) -> *const Self::Target {
        Self::as_ptr(self)
    }
}

impl<T: ?Sized> Pointer for &T {
    type Target = T;

    fn pointer(&self) -> *const Self::Target {
        *self
    }
}
