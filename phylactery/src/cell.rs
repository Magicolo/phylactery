use crate::{Binding, lich, soul};

#[repr(transparent)]
pub struct Cell(core::cell::Cell<u32>);
pub type Lich<T> = lich::Lich<T, Cell>;
pub type Soul<P> = soul::Soul<P, Cell>;

unsafe impl Binding for Cell {
    const NEW: Self = Self(core::cell::Cell::new(0));

    fn sever<const FORCE: bool>(&self) -> bool {
        match self.0.get() {
            0 | u32::MAX => {
                self.0.set(u32::MAX);
                true
            }
            value if FORCE => panic!("{value} `Lich<T>`es have not been redeemed"),
            _ => false,
        }
    }

    fn redeem(&self) {}

    fn count(&self) -> u32 {
        match self.0.get() {
            0 | u32::MAX => 0,
            count => count,
        }
    }

    fn increment(&self) -> u32 {
        let value = self.0.get();
        assert!(value < u32::MAX - 1);
        self.0.set(value + 1);
        value
    }

    fn decrement(&self) -> u32 {
        let value = self.0.get();
        debug_assert!(value > 0);
        self.0.set(value - 1);
        value
    }
}
