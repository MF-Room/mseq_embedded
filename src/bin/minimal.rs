#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use mseq_hardware as _; // global logger + panicking-behavior + memory layout

#[rtic::app(
    device = stm32f4xx_hal::pac,
    // TODO: Replace the `FreeInterrupt1, ...` with free interrupt vectors if software tasks are used
    // You can usually find the names of the interrupt vectors in the some_hal::pac::interrupt enum.
    dispatchers = [TIM3],
    peripherals = true,
)]
mod app {   
    use rtic_monotonics::systick::prelude::*;
    use stm32f4xx_hal::{
        interrupt,
        pac::{self, USART1},
        prelude::*,
        rcc::AHB1,
        serial::{
            self, config::{DmaConfig, StopBits::STOP1}, Config, Serial
        },
    };
    
    systick_monotonic!(Mono, 100);

    #[shared]
    struct Shared {
    }

    #[local]
    struct Local {
        serial: Serial<USART1>
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");
        
        // Midi connection
        let rcc = cx.device.RCC.constrain();
        let clocks = rcc.cfgr.freeze();
        let gpioa = cx.device.GPIOA.split();
        let rx_1 = gpioa.pa10.into_alternate();
        let tx_1 = gpioa.pa9.into_alternate();

        let mut serial = Serial::new(
            cx.device.USART1,
            (tx_1, rx_1),
            Config::default()
                .baudrate(31250.bps())
                .wordlength_8()
                .parity_none()
                .stopbits(STOP1)
                .dma(DmaConfig::None),
            &clocks,
        ).unwrap();

        serial.listen(serial::Event::RxNotEmpty);

        // Timer
        Mono::start(cx.core.SYST, 12_000_000);

        // Start
        task1::spawn().ok();

        rtic::pend(interrupt::USART1);

        (
            Shared {
            },
            Local {
                serial
            },
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("idle");

        loop {
            continue;
        }
    }

    // Sleep demo task
    #[task(priority = 1)]
    async fn task1(_cx: task1::Context) {
        defmt::info!("Hello from task1!");
        Mono::delay(5000.millis()).await;
        defmt::info!("Hello after wait!");
    }

    // Midi interrupt
    #[task(binds = USART1, priority = 2, local=[serial])]
    fn midi_int(cx: midi_int::Context) {
        let serial = cx.local.serial;
        match serial.read() {
            Ok(b) => defmt::info!("Received: {}", b),
            Err(_) => defmt::info!("Serial is empty"),
        }
    }
}
