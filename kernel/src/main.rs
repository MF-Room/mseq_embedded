#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

extern crate alloc;
mod driver;
mod heap;
mod midi_connection;
mod rtt_logger;
mod screen;

use panic_rtt_target as _;

#[rtic::app(
    device = stm32f4xx_hal::pac,
    // TODO: Replace the `FreeInterrupt1, ...` with free interrupt vectors if software tasks are used
    // You can usually find the names of the interrupt vectors in the some_hal::pac::interrupt enum.
    dispatchers = [ADC],
    peripherals = true,
)]

mod app {
    use alloc::vec;
    use alloc::vec::Vec;
    use log::{debug, trace};
    use mseq::MidiController;
    use rtic_monotonics::systick::prelude::*;
    use stm32f4xx_hal::{
        pac::USART1,
        prelude::*,
        rtc::Rtc,
        serial::{
            config::{DmaConfig, StopBits::STOP1},
            Config, Rx, Serial,
        },
    };

    use crate::midi_connection::MidiOut;
    use crate::rtt_logger;
    use crate::screen;
    use crate::{heap, rtt_logger::RttLogger};
    use user::conductor;

    //TODO: understand and add comment
    systick_monotonic!(Mono, 100);

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        lcd: screen::Lcd,
        rx: Rx<USART1>,
        rtc: Rtc,
        mseq_ctx: mseq::Context<MidiOut>,
        conductor: conductor::UserConductor,
        clock_period: u32,
    }

    #[init(local = [logger: RttLogger = RttLogger {level: log::LevelFilter::Off} ])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        rtt_logger::RttLogger::init(cx.local.logger, log::LevelFilter::Trace);

        trace!("Init");

        // Initilialize allocator
        heap::allocator_init();

        // Serial connection
        let rcc = cx.device.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).freeze();
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
                .dma(DmaConfig::None),
            &clocks,
        )
        .expect("Failed to initialize serial");
        let (mut tx, mut rx) = serial.split();
        rx.listen();

        tx.write(0xfa).unwrap();

        // lcd screen
        let gpiob = cx.device.GPIOB.split();
        let i2c = stm32f4xx_hal::i2c::I2c::new(
            cx.device.I2C1,
            (gpiob.pb6, gpiob.pb7),
            stm32f4xx_hal::i2c::Mode::standard(50.kHz()),
            &clocks,
        );

        let delay = cx.device.TIM3.delay_us(&clocks);

        // Test allocator
        let mut v: Vec<u32> = vec![];
        v.push(1);

        // MidiOut
        let midi_out = MidiOut::new(tx);

        // Think about user interface for this
        let conductor = conductor::UserConductor::new();
        let midi_controller = MidiController::new(midi_out);
        let mseq_ctx = mseq::Context::new(midi_controller);

        // Clock
        let mut rtc = Rtc::new(cx.device.RTC, &mut cx.device.PWR);
        let clock_period = mseq_ctx.get_period_us() as u32;
        rtc.enable_wakeup(clock_period.micros::<1, 1_000_000>().into());
        rtc.listen(&mut cx.device.EXTI, stm32f4xx_hal::rtc::Event::Wakeup);

        trace!("Init over");

        (
            Shared {},
            Local {
                lcd: screen::Lcd::new(i2c, delay),
                rx,
                rtc,
                mseq_ctx,
                conductor,
                clock_period,
            },
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            continue;
        }
    }

    #[task(binds = RTC_WKUP, priority = 2, local = [rtc, mseq_ctx, conductor, clock_period])]
    fn clock(mut cx: clock::Context) {
        trace!("Clock");

        cx.local
            .rtc
            .clear_interrupt(stm32f4xx_hal::rtc::Event::Wakeup);

        let mseq_ctx = &mut cx.local.mseq_ctx;
        mseq_ctx.process_post_tick();
        mseq_ctx.process_pre_tick(cx.local.conductor);

        // If clock changed, update callback timing
        if mseq_ctx.get_period_us() as u32 != *cx.local.clock_period {
            *cx.local.clock_period = mseq_ctx.get_period_us() as u32;
            cx.local
                .rtc
                .enable_wakeup(cx.local.clock_period.micros::<1, 1_000_000>().into());
        }
    }

    // Midi interrupt
    #[task(binds = USART1, priority = 2, local=[rx])]
    fn midi_int(cx: midi_int::Context) {
        let serial = cx.local.rx;
        match serial.read() {
            Ok(b) => {
                debug!("{b} received")
            } // defmt::info!("Received: {}", b),
            Err(_) => {} //defmt::info!("Serial is empty"),
        }
    }
}
