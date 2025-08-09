pub trait Sever {
    fn sever(&mut self) -> bool;
}

impl<T> Sever for Option<T> {
    fn sever(&mut self) -> bool {
        self.take().is_some()
    }
}
