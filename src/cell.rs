use crate::{Bind, Sever, shroud::Shroud};
use core::{
    cell::{Ref, RefCell},
    ptr::{self, NonNull},
};
use std::rc::{Rc, Weak};

pub struct Cell;

pub type Soul<'a> = crate::Soul<'a, Cell>;
pub type Lich<T> = crate::Lich<T, Cell>;
pub type Guard<'a, T> = crate::Guard<'a, T, Cell>;

unsafe impl<'a, T: ?Sized + 'a> Send for Lich<T> where Rc<RefCell<Option<&'a T>>>: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Lich<T> where Rc<RefCell<Option<&'a T>>>: Sync {}

impl<T: Sever + ?Sized> Sever for Rc<RefCell<T>> {
    fn sever(&mut self) -> bool {
        self.borrow_mut().sever()
    }

    fn try_sever(&mut self) -> Option<bool> {
        self.try_borrow_mut().ok()?.try_sever()
    }
}

impl<T: Sever + ?Sized> Sever for Weak<RefCell<T>> {
    fn sever(&mut self) -> bool {
        self.upgrade().is_some_and(|mut strong| strong.sever())
    }

    fn try_sever(&mut self) -> Option<bool> {
        self.upgrade()?.try_sever()
    }
}

impl Bind for Cell {
    type Refer<'a, T: ?Sized + 'a> = Ref<'a, Option<NonNull<T>>>;
    type Strong<T: ?Sized> = Rc<RefCell<Option<NonNull<T>>>>;
    type Weak<'a> = Weak<RefCell<dyn Sever + 'a>>;

    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Strong<S>, Self::Weak<'a>) {
        let strong = Rc::new(RefCell::new(Some(S::shroud(value))));
        let weak = Rc::downgrade(&strong);
        (strong, weak)
    }

    fn are_bound<'a, T: ?Sized>(strong: &Self::Strong<T>, weak: &Self::Weak<'a>) -> bool {
        ptr::addr_eq(Rc::as_ptr(strong), Weak::as_ptr(weak))
    }

    fn is_bound_weak(weak: &Self::Weak<'_>) -> bool {
        Weak::strong_count(weak) > 0
    }

    fn is_bound_strong<T: ?Sized>(strong: &Self::Strong<T>) -> bool {
        Rc::weak_count(strong) > 0
    }
}

impl<T: ?Sized> Lich<T> {
    pub fn borrow(&self) -> Option<Guard<'_, T>> {
        // `try_borrow` can be used here because only the `sever` operation calls
        // `borrow_mut`, at which point, the value must not be observable
        let guard = self.0.try_borrow().ok()?;
        if guard.is_some() {
            Some(crate::Guard(guard))
        } else {
            None
        }
    }
}

pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> (Lich<S>, Soul<'a>) {
    crate::ritual(value)
}

pub fn redeem<'a, T: ?Sized + 'a>(lich: Lich<T>, soul: Soul<'a>) -> Option<(Lich<T>, Soul<'a>)> {
    unsafe { crate::redeem(lich, soul) }
}
