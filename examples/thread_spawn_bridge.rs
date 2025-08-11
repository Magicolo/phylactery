/// Trivially reimplement `thread::scope` in a more powerful way.
///
/// Contrarily to other `scope` solutions, here, the captured reference can be
/// returned (as a `Soul<'a>`) while the threads continue to execute.
#[cfg(feature = "lock")]
pub mod thread_spawn_bridge {
    use core::num::NonZeroUsize;
    use phylactery::lock::{Soul, ritual};
    use std::thread;

    pub fn broadcast<F: Fn(usize) + Send + Sync>(
        parallelism: NonZeroUsize,
        function: &F,
    ) -> Soul<'_> {
        let (lich, soul) = ritual::<_, dyn Fn(usize) + Send + Sync>(function);
        // Spawn a bunch of threads that will all call `F`.
        for index in 0..parallelism.get() {
            let lich = lich.clone();
            // The non-static function `F` crosses a `'static` boundary protected by the
            // `Lich<T>`.
            thread::spawn(move || {
                // Borrowing may fail if the `Soul<'a>` has been dropped/severed.
                if let Some(guard) = lich.borrow() {
                    // Call the non-static function.
                    guard(index);
                }
                // Allow the `Guard` and `Lich<T>` to drop such that the
                // `Soul<'a>` can complete its `Soul::sever`.
            });
        }

        // The `Soul<'a>` continues to track the captured `'a` reference and will
        // guarantee that it becomes inaccessible when it itself drops.
        //
        // Note that this may block this thread if there still are active borrows at the
        // time of drop.
        //
        // Note that the `Lich<T>`es do not need be `redeem`ed.
        soul
    }
}

fn main() {
    #[cfg(feature = "lock")]
    thread_spawn_bridge::broadcast(
        std::thread::available_parallelism().unwrap_or(core::num::NonZeroUsize::MIN),
        &|index| println!("{index}"),
    );
}
