use core::sync::atomic::{AtomicUsize, Ordering};
use defmt_brtt as _; // global logger
use panic_probe as _;

#[defmt::panic_handler]
fn panic() -> ! {
    // To avoid go through the core::panic machinery and that may reduce code size
    panic_probe::hard_fault();
}

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", COUNT.fetch_add(1, Ordering::Relaxed));

/// Terminates the application and makes `probe-rs` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        // The BKPT instruction causes the processor to enter Debug state
        cortex_m::asm::bkpt();
    }
}
