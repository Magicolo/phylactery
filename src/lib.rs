/*!
# Phylactery

This library offers a safe wrapper around lifetime extension shenanigans by splitting a `&'a T` into a
`Lich<dyn T + 'b>` (`'b` can be any chosen lifetime) and a `Soul<'a>` which tracks the original lifetime. On
drop of the `Soul` or on calling `Soul::sever`, it is guaranteed that the captured reference is also dropped, thus
inaccessible from a remaining `Lich`.
!*/

#[cfg(feature = "cell")]
pub mod cell;
#[cfg(feature = "lock")]
pub mod lock;
#[cfg(feature = "raw")]
pub mod raw;
pub mod sever;
pub mod shroud;

/*
TODO:
- Myth: The [`Lich<T>`] is a being that forfeited its [`Soul<'a>`] through a ritual in order to become undead.
- Rust: The [`Lich<T>`] is a `&'a T` that forfeited its lifetime through some unsafe code in order to become `'static`.

- 2 scenarios come to mind for usage of the `Lich`:
    - thread local scoped state
        - for example, a profiler stores some thread-local data that lives on the stack in a static variable such that
        the profiler object does not need to be carried around
        - similarly for a logger scope a log prefix or values that live on the stack
        - normally, one would need to somehow move the values from the stack to a thread-local storage, since there is
        a `'static` requirement and this would simply not be possible for non-static values
    - scoped `thread/task::spawn`
        - since the `Lich` can have any chosen lifetime, it can choose `'static` in order to cross a thread boundary
        using a regular `spawn`, thus bridging and/or expanding the capabilities of threading libraries

- It is never safe to give a `&'static T` from a `&'a T`, even within a closure, since static references may escape the closure.
- Even when hidden within a data handle, it must be known that it the handle may escape the closure.
- Should it be possible to `Clone` the `Lich`?
    - It can be trivially cloned because of its `Arc`.
    - It may be unclear that using `borrow_mut` takes a `write` lock; thus can dead lock.
- What about not supporting `borrow_mut`?
- What about requiring the `Lich<T>` and its `Soul<'a>` to be reunited?

Evil things that must be impossible:
- Storing an extended lifetime value in a `static` field.
    - Unsized values prevent most of the issues...
    - `Split` and `SplitMut` traits should be `unsafe`.
    static EVIL: OnceLock<Arc<Mutex<dyn Fn() + Send + Sync + 'static>>> = OnceLock::new();
    let (mut data, life) = split_mut::<dyn Fn() + Send + Sync + 'static>(f);
    let evil = EVIL.get_or_init(|| Arc::new(Mutex::new(|| {})));
    swap(
        data.borrow_mut().deref_mut(),
        evil.lock().unwrap().deref_mut(),
    );
- *NEVER* allow giving out a `static` reference to the inner type even though its lifetime is `'static`.
*/
use crate::shroud::Shroud;
use core::{mem::ManuallyDrop, ops::Deref, ptr};

pub trait Order {
    type Weak<'a>;
    type Strong<T: ?Sized>;
    type Refer<'a, T: ?Sized + 'a>;

    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Strong<S>, Self::Weak<'a>);
    fn are_bound<T: ?Sized>(strong: &Self::Strong<T>, weak: &Self::Weak<'_>) -> bool;
    fn is_bound_weak(weak: &Self::Weak<'_>) -> bool;
    fn is_bound_strong<T: ?Sized>(strong: &Self::Strong<T>) -> bool;
    fn try_sever_strong<T: ?Sized>(_: &Self::Strong<T>) -> Option<bool>;
    fn try_sever_weak(_: &Self::Weak<'_>) -> Option<bool>;
    fn sever_strong<T: ?Sized>(_: &Self::Strong<T>) -> bool;
    fn sever_weak(_: &Self::Weak<'_>) -> bool;
}

pub struct Soul<'a, O: Order + ?Sized>(pub(crate) O::Weak<'a>);
pub struct Lich<T: ?Sized, O: Order + ?Sized>(pub(crate) O::Strong<T>);
pub struct Guard<'a, T: ?Sized + 'a, O: Order + ?Sized>(pub(crate) O::Refer<'a, T>);

unsafe impl<T: Send + ?Sized, O: Order + ?Sized> Send for Lich<T, O> {}
unsafe impl<T: Sync + ?Sized, O: Order + ?Sized> Sync for Lich<T, O> {}

impl<T: ?Sized, O: Order + ?Sized> Lich<T, O> {
    pub fn is_bound(&self) -> bool {
        O::is_bound_strong(&self.0)
    }

    pub fn sever(self) -> bool {
        O::sever_strong(&self.0)
    }

    pub fn try_sever(self) -> Result<bool, Self> {
        O::try_sever_strong(&self.0).ok_or(self)
    }
}

impl<O: Order + ?Sized> Soul<'_, O> {
    pub fn is_bound(&self) -> bool {
        O::is_bound_weak(&self.0)
    }

    pub fn sever(self) -> bool {
        O::sever_weak(&self.0)
    }

    pub fn try_sever(self) -> Result<bool, Self> {
        O::try_sever_weak(&self.0).ok_or(self)
    }
}

impl<T: ?Sized, O: Order + ?Sized> Drop for Lich<T, O> {
    fn drop(&mut self) {
        O::sever_strong(&self.0);
    }
}

impl<O: Order + ?Sized> Drop for Soul<'_, O> {
    fn drop(&mut self) {
        O::sever_weak(&self.0);
    }
}

impl<'a, T: ?Sized, O: Order<Refer<'a, T>: Deref<Target = Option<*const T>>> + ?Sized> Deref
    for Guard<'a, T, O>
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &**self.0.deref().as_ref().unwrap_unchecked() }
    }
}

impl<'a, T: ?Sized, O: Order<Refer<'a, T>: AsRef<Option<*const T>>> + ?Sized> AsRef<T>
    for Guard<'a, T, O>
{
    fn as_ref(&self) -> &T {
        unsafe { &**self.0.as_ref().as_ref().unwrap_unchecked() }
    }
}

fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a, O: Order + ?Sized>(
    value: &'a T,
) -> (Lich<S, O>, Soul<'a, O>) {
    let (strong, weak) = O::bind(value);
    (Lich(strong), Soul(weak))
}

unsafe fn redeem<'a, T: ?Sized + 'a, O: Order + ?Sized>(
    lich: Lich<T, O>,
    soul: Soul<'a, O>,
) -> Option<(Lich<T, O>, Soul<'a, O>)> {
    if O::are_bound(&lich.0, &soul.0) {
        let lich = ManuallyDrop::new(lich);
        unsafe { ptr::read(&lich.0) };
        let soul = ManuallyDrop::new(soul);
        unsafe { ptr::read(&soul.0) };
        None
    } else {
        Some((lich, soul))
    }
}
