/// Terminates the application and makes `probe-rs` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        // The BKPT instruction causes the processor to enter Debug state
        cortex_m::asm::bkpt();
    }
}
