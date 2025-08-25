/// Trivially reimplement [`thread::scope`] in a more powerful way.
///
/// Contrary to other `scope` solutions, here, the captured reference can be
/// returned (as a [`Soul<P>`]) while the threads continue to execute.
#[cfg(feature = "lock")]
pub mod thread_spawn_bridge {
    use core::num::NonZeroUsize;
    use phylactery::lock::Soul;
    use std::thread;

    pub fn broadcast<F: Fn(usize) + Send + Sync>(
        parallelism: NonZeroUsize,
        function: &F,
    ) -> Soul<&F, Box<u32>> {
        let soul = Soul::new(function);
        // Spawn a bunch of threads that will all call `F`.
        for index in 0..parallelism.get() {
            let lich = soul.bind::<dyn Fn(usize) + Send + Sync>();
            // The non-static function `F` crosses a `'static` boundary protected by the
            // `Lich`.
            thread::spawn(move || {
                // Call the non-static function.
                lich(index);
            });
        }

        // The `Soul` continues to track the captured `&F` reference and will
        // guarantee that it becomes inaccessible when it itself drops.
        //
        // If a `Lich` bound to this `Soul` still lives at the time of drop,
        // `<Soul as Drop>::drop` will block until all `Lich`es are dropped.
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
