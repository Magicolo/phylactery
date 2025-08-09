use crate::{Order, Sever, shroud::Shroud};
use core::{
    cell::{Ref, RefCell},
    ptr,
};
use std::rc::{Rc, Weak};

pub struct Cell;

pub type Soul<'a> = crate::Soul<'a, Cell>;
pub type Lich<T> = crate::Lich<T, Cell>;
pub type Guard<'a, T> = crate::Guard<'a, T, Cell>;

impl Order for Cell {
    type Refer<'a, T: ?Sized + 'a> = Ref<'a, Option<*const T>>;
    type Strong<T: ?Sized> = Rc<RefCell<Option<*const T>>>;
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

    fn try_sever_strong<T: ?Sized>(strong: &Self::Strong<T>) -> Option<bool> {
        Some(strong.try_borrow_mut().ok()?.sever())
    }

    fn try_sever_weak(weak: &Self::Weak<'_>) -> Option<bool> {
        Some(weak.upgrade()?.try_borrow_mut().ok()?.sever())
    }

    fn sever_strong<T: ?Sized>(strong: &Self::Strong<T>) -> bool {
        strong.borrow_mut().sever()
    }

    fn sever_weak(weak: &Self::Weak<'_>) -> bool {
        match weak.upgrade() {
            Some(cell) => cell.borrow_mut().sever(),
            None => false,
        }
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
