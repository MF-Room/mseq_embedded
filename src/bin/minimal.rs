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
    const BUFFER_SIZE: usize = 3;

    use rtic_monotonics::systick::prelude::*;
    use stm32f4xx_hal::{
        dma::{self, DmaEvent, Transfer},
        pac::{DMA2, USART1},
        prelude::*,
        serial::{
            config::{DmaConfig, StopBits::STOP1},
            Config, Serial,
        },
    };

    systick_monotonic!(Mono, 100);

    #[shared]
    struct Shared {
        transfer: stm32f4xx_hal::dma::Transfer<
            dma::Stream2<DMA2>,
            4,
            stm32f4xx_hal::serial::Rx<stm32f4xx_hal::pac::USART1>,
            stm32f4xx_hal::dma::PeripheralToMemory,
            &'static mut [u8; BUFFER_SIZE],
        >,
    }

    #[local]
    struct Local {
        buffer: Option<&'static mut [u8; BUFFER_SIZE]>,
    }

    #[init(local = [buf1: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE], buf2: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE]])]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");

        // Timer
        Mono::start(cx.core.SYST, 12_000_000);

        // Serial connection
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
                .dma(DmaConfig::TxRx),
            &clocks,
        )
        .expect("Failed to initialize serial");

        // Setup dma
        let mut dma = dma::StreamsTuple::new(cx.device.DMA2);
        dma.2.listen(DmaEvent::TransferComplete);

        // Init transfer
        let (_, rx) = serial.split();
        let mut transfer = Transfer::init_peripheral_to_memory(
            dma.2,
            rx,
            cx.local.buf1,
            None,
            dma::config::DmaConfig::default()
                .transfer_complete_interrupt(true)
                .memory_increment(true),
        );

        // Start transfer
        transfer.start(|_| {});

        (
            Shared { transfer },
            Local {
                buffer: Some(cx.local.buf2),
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

    #[task(priority = 1)]
    async fn sleep5s(_cx: sleep5s::Context) {
        defmt::info!("Sleep start");
        Mono::delay(5000.millis()).await;
        defmt::info!("Sleep stop");
    }

    #[task(binds = DMA2_STREAM2, priority = 2, shared = [transfer], local = [buffer])]
    fn midi_dma2(cx: midi_dma2::Context) {
        let mut shared = cx.shared;
        let local = cx.local;
        let buffer = shared.transfer.lock(|transfer| {
            let (buffer, _) = transfer
                .next_transfer(local.buffer.take().expect("Failed to take buffer"))
                .expect("Failed to get dma buffer");
            buffer
        });
        defmt::info!(
            "dma buffer full: {}, {}, {}",
            buffer[0],
            buffer[1],
            buffer[2],
        );
        *local.buffer = Some(buffer);
    }
}
