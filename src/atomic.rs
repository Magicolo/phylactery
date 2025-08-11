use crate::{Binding, Sever, TrySever, shroud::Shroud};
use atomic_wait::{wait, wake_one};
use core::{
    borrow::Borrow,
    ptr::{NonNull, addr_eq},
    sync::atomic::{AtomicU32, Ordering},
};

pub struct Atomic;

pub type Soul<'a> = crate::Soul<'a, Atomic>;
pub type Lich<T> = crate::Lich<T, Atomic>;
pub type Pair<'a, T> = crate::Pair<'a, T, Atomic>;

pub struct Data<T: ?Sized>(NonNull<T>, &'static AtomicU32);
pub struct Life<'a>(&'a AtomicU32);

unsafe impl<'a, T: ?Sized + 'a> Send for Data<T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Data<T> where &'a T: Sync {}

impl<T: ?Sized> TrySever for Data<T> {
    fn try_sever(&mut self) -> Option<bool> {
        None
    }
}

impl<T: ?Sized> Clone for Data<T> {
    fn clone(&self) -> Self {
        self.1.fetch_add(1, Ordering::Relaxed);
        Self(self.0, self.1)
    }
}

impl<T: ?Sized> Drop for Data<T> {
    fn drop(&mut self) {
        if self.1.fetch_sub(1, Ordering::Relaxed) == 1 {
            wake_one(self.1);
        }
    }
}

impl Sever for Life<'_> {
    fn sever(&mut self) -> bool {
        sever::<true>(self.0).is_some_and(|value| value)
    }
}

impl TrySever for Life<'_> {
    fn try_sever(&mut self) -> Option<bool> {
        sever::<false>(self.0)
    }
}

impl Binding for Atomic {
    type Data<T: ?Sized> = Data<T>;
    type Life<'a> = Life<'a>;

    fn are_bound<T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'_>) -> bool {
        addr_eq(data.1, life.0)
    }

    fn is_life_bound(life: &Self::Life<'_>) -> bool {
        bound(life.0)
    }

    fn is_data_bound<T: ?Sized>(data: &Self::Data<T>) -> bool {
        bound(data.1)
    }
}

impl<T: ?Sized> Borrow<T> for Lich<T> {
    fn borrow(&self) -> &T {
        self.borrow()
    }
}

impl<T: ?Sized> Lich<T> {
    #[allow(clippy::should_implement_trait)]
    pub fn borrow(&self) -> &T {
        unsafe { self.0.0.as_ref() }
    }
}

pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized>(
    value: &'a T,
    slot: &'a mut u32,
) -> Pair<'a, S> {
    *slot = 1;
    // # Safety
    // `slot` is trivially valid as an `AtomicU32` and since it is a mutable
    // borrow, it is exclusively owned by this function
    let count = unsafe { AtomicU32::from_ptr(slot) };
    let pointer = S::shroud(value);
    (crate::Lich(Data(pointer, count)), crate::Soul(Life(count)))
}

pub fn redeem<'a, T: ?Sized + 'a>(
    lich: Lich<T>,
    soul: Soul<'a>,
) -> Result<Option<Soul<'a>>, Pair<'a, T>> {
    crate::redeem::<_, _, true>(lich, soul)
}

fn sever<const WAIT: bool>(count: &AtomicU32) -> Option<bool> {
    loop {
        match count.compare_exchange(0, u32::MAX, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(0) => break Some(true),
            Ok(u32::MAX) | Err(u32::MAX) => break Some(false),
            Ok(value) | Err(value) if WAIT => wait(count, value),
            Ok(_) | Err(_) => break None,
        }
    }
}

fn bound(count: &AtomicU32) -> bool {
    let count = count.load(Ordering::Relaxed);
    count > 0 && count < u32::MAX
}
