/// Trivially reimplement [`thread::scope`] in a more powerful way.
///
/// Contrary to other `scope` solutions, here, the captured reference can be
/// returned (as a [`Soul<T>`]) while the threads continue to execute.
#[cfg(all(feature = "atomic", feature = "shroud"))]
pub mod thread_spawn_bridge {
    use core::{num::NonZeroUsize, pin::Pin};
    use phylactery::atomic::Soul;
    use std::thread;

    pub fn broadcast<F: Fn(usize) + Send + Sync>(
        parallelism: NonZeroUsize,
        function: F,
    ) -> Pin<Box<Soul<F>>> {
        // Pin the `Soul` to the heap to be able to return it.
        let soul = Box::pin(Soul::new(function));
        // Spawn a bunch of threads that will all call `F`.
        for index in 0..parallelism.get() {
            // `Soul::bind` requires a pinning.
            let lich = soul.as_ref().bind::<dyn Fn(usize) + Send + Sync>();
            // The non-static function `F` crosses a `'static` boundary protected by the
            // `Lich` and is called on another thread. `Send/Sync` requirements still apply.
            thread::spawn(move || lich(index));
        }

        // The `Soul` continues to track the captured `F` and will guarantee that it
        // becomes inaccessible when it itself drops.
        //
        // If a `Lich` bound to this `Soul` still lives at the time of drop,
        // `<Soul as Drop>::drop` will block until all `Lich`es are dropped.
        soul
    }
}

fn main() {
    #[cfg(all(feature = "atomic", feature = "shroud"))]
    thread_spawn_bridge::broadcast(
        std::thread::available_parallelism().unwrap_or(core::num::NonZeroUsize::MIN),
        &|index| println!("{index}"),
    );
}
