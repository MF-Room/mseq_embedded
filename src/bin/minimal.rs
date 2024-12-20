#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use mseq_hardware as _; // global logger + panicking-behavior + memory layout

#[rtic::app(
    device = stm32f4xx_hal::pac,
    // TODO: Replace the `FreeInterrupt1, ...` with free interrupt vectors if software tasks are used
    // You can usually find the names of the interrupt vectors in the some_hal::pac::interrupt enum.
    dispatchers = [],
    peripherals = true,
)]
mod app {
    use rtic_monotonics::systick::prelude::*;
    use stm32f4xx_hal::{
        dma::{self, DmaEvent, Transfer},
        interrupt,
        pac::{otg_fs_device::diep::txfsts, DMA2, USART1},
        prelude::*,
        serial::{
            config::{DmaConfig, StopBits::STOP1},
            Config, Serial,
        },
    };

    systick_monotonic!(Mono, 100);

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        recv: stm32f4xx_hal::dma::Transfer<
            dma::StreamX<DMA2,2> ,
            4,
            stm32f4xx_hal::serial::Rx<stm32f4xx_hal::pac::USART1>,
            stm32f4xx_hal::dma::PeripheralToMemory,
            &'static mut [u8; 128],
        >,
    }

    #[init(local = [buf: [u8; 128] = [0; 128]])]
    fn init(cx: init::Context) -> (Shared, Local) {
        cx.device.RCC.ahb1enr().modify(|_, w| w.dma2en().enabled());

        defmt::info!("init");

        // Midi connection
        let rcc = cx.device.RCC.constrain();
        let clocks = rcc.cfgr.freeze();
        let gpioa = cx.device.GPIOA.split();
        let rx_1 = gpioa.pa10.into_alternate();
        let tx_1 = gpioa.pa9.into_alternate();

        let serial: Serial<USART1> = Serial::new(
            cx.device.USART1,
            (tx_1, rx_1),
            Config::default()
                .baudrate(31250.bps())
                .wordlength_8()
                .parity_none()
                .stopbits(STOP1)
                .dma(DmaConfig::Rx),
            &clocks,
        )
        .unwrap();

        let (tx, rx) = serial.split();

        let mut dma = dma::StreamsTuple::new(cx.device.DMA2);
        dma.2.listen(DmaEvent::TransferComplete);

        // let serial_dma = serial.use_dma_rx(dma.2);

        // Timer
        Mono::start(cx.core.SYST, 12_000_000);

        // Start
        rtic::pend(interrupt::DMA2_STREAM2);
        let recv = Transfer::init_peripheral_to_memory(
            dma.2,
            rx,
            cx.local.buf,
            None,
            dma::config::DmaConfig::default().transfer_complete_interrupt(true),
        );

        (Shared {}, Local { recv })
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("idle");

        loop {
            continue;
        }
    }

    #[task(binds = DMA2_STREAM2, priority = 2, local = [recv])]
    fn midi_dma2(cx: midi_dma2::Context) {
        defmt::info!("dma stream 2 triggered");
        // cx.local.recv.dk
    }
}
