#![no_main]
#![no_std]

extern crate alloc;
mod heap;
mod midi_connection;
mod midi_input;
mod rtt_logger;

use panic_rtt_target as _;

#[rtic::app(
    device = stm32f4xx_hal::pac,
    // TODO: Replace the `FreeInterrupt1, ...` with free interrupt vectors if software tasks are used
    // You can usually find the names of the interrupt vectors in the some_hal::pac::interrupt enum.
    dispatchers = [ADC, DMA1_STREAM0],
    peripherals = true,
)]

mod app {
    use log::{debug, error, trace, warn};
    use mseq_core::*;
    use rtic_monotonics::systick::prelude::*;
    use rtic_sync::{
        make_signal,
        signal::{Signal, SignalReader, SignalWriter},
    };
    use stm32f4xx_hal::{
        pac::USART1,
        prelude::*,
        rtc::Rtc,
        serial::{
            Config, Rx, Serial,
            config::{DmaConfig, StopBits::STOP1},
        },
    };

    use crate::midi_connection::MidiOut;
    use crate::midi_input::MidiInputHandler;
    use crate::rtt_logger;
    use crate::{heap, rtt_logger::RttLogger};
    use user::conductor;

    //TODO: understand and add comment
    systick_monotonic!(Mono, 100);

    #[shared]
    struct Shared {
        conductor: conductor::UserConductor,
        input_queue: InputQueue,
        midi_controller: MidiController<MidiOut>,
        mseq_ctx: mseq_core::Context,
        display_text: driver::DisplayText,
    }

    #[local]
    struct Local {
        rx: Rx<USART1>,
        rtc: Rtc,
        clock_period: u32,
        midi_input_handler: MidiInputHandler,
        input_signal_writer: SignalWriter<'static, ()>,
        display: driver::Lcd,
    }

    #[init(local = [logger: RttLogger = RttLogger {level: log::LevelFilter::Off} ])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        rtt_logger::RttLogger::init(cx.local.logger, log::LevelFilter::Debug);

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
        let display = driver::Lcd::new(i2c, delay);

        // MidiOut
        let midi_out = MidiOut::new(tx);

        let conductor = conductor::UserConductor::new();
        let midi_controller = MidiController::new(midi_out);
        let mseq_ctx = mseq_core::Context::new();

        // Clock
        let mut rtc = Rtc::new(cx.device.RTC, &mut cx.device.PWR);
        let clock_period = mseq_ctx.get_period_us() as u32;
        rtc.enable_wakeup(clock_period.micros::<1, 1_000_000>().into());
        rtc.listen(&mut cx.device.EXTI, stm32f4xx_hal::rtc::Event::Wakeup);

        // Input Queue
        let input_queue = InputQueue::new();

        // Input Signal
        let (w, r) = make_signal!(());
        handle_input::spawn(r).unwrap();

        trace!("Init over");

        (
            Shared {
                conductor,
                input_queue,
                midi_controller,
                mseq_ctx,
                display_text: driver::DisplayText::default(),
            },
            Local {
                rx,
                rtc,
                clock_period,
                midi_input_handler: MidiInputHandler::new(),
                input_signal_writer: w,
                display,
            },
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            continue;
        }
    }

    #[task(binds = RTC_WKUP, priority = 3, local = [rtc, clock_period], shared = [conductor, midi_controller, mseq_ctx, display_text])]
    fn clock(mut cx: clock::Context) {
        trace!("Clock");

        // Clear clock interrupt flag
        cx.local
            .rtc
            .clear_interrupt(stm32f4xx_hal::rtc::Event::Wakeup);

        // mseq logic
        // post tick
        (&mut cx.shared.mseq_ctx, &mut cx.shared.midi_controller)
            .lock(|mseq_ctx, midi_controller| mseq_ctx.process_post_tick(midi_controller));

        let mut current_step = 0;
        // pre tick
        (
            &mut cx.shared.mseq_ctx,
            &mut cx.shared.midi_controller,
            &mut cx.shared.conductor,
        )
            .lock(|mseq_ctx, midi_controller, conductor| {
                mseq_ctx.process_pre_tick(conductor, midi_controller);
                current_step = mseq_ctx.get_step();
            });

        // Screen is refreshed on each beat
        if current_step % 24 == 1 {
            // Update display text
            (
                &mut cx.shared.conductor,
                &mut cx.shared.mseq_ctx,
                &mut cx.shared.display_text,
            )
                .lock(|conductor, ctx, display_text| {
                    *display_text = conductor.display_text(ctx);
                });
            match update_display::spawn() {
                Ok(_) => (),
                Err(_) => warn!("Display update skipped"),
            }
        }

        // If clock changed, update callback timing
        cx.shared.mseq_ctx.lock(|mseq_ctx| {
            if mseq_ctx.get_period_us() as u32 != *cx.local.clock_period {
                *cx.local.clock_period = mseq_ctx.get_period_us() as u32;
            }

            cx.local
                .rtc
                .enable_wakeup(cx.local.clock_period.micros::<1, 1_000_000>().into());
        })
    }

    // Midi interrupt
    #[task(binds = USART1, priority = 4, local=[rx, midi_input_handler, input_signal_writer], shared = [input_queue])]
    fn midi_int(mut cx: midi_int::Context) {
        let serial = cx.local.rx;
        match serial.read() {
            Ok(b) => {
                debug!("{b} received");
                cx.local
                    .midi_input_handler
                    .process_byte(b)
                    .map(|midi_message| {
                        cx.shared
                            .input_queue
                            .lock(|input_queue| input_queue.push_back(midi_message))
                    });
            }
            Err(_) => error!("Serial error"),
        }

        cx.local.input_signal_writer.write(());
    }

    #[task(priority = 2, shared = [mseq_ctx, conductor, midi_controller, input_queue])]
    async fn handle_input(
        mut cx: handle_input::Context,
        mut input_signal_reader: SignalReader<'static, ()>,
    ) {
        let ctx = &mut cx.shared.mseq_ctx;
        let conductor = &mut cx.shared.conductor;
        let controller = &mut cx.shared.midi_controller;
        let input_queue = &mut cx.shared.input_queue;

        loop {
            input_signal_reader.wait().await;

            let mut inputs = InputQueue::new();
            input_queue.lock(|input_queue| inputs = core::mem::take(input_queue));
            (&mut *ctx, &mut *conductor, &mut *controller).lock(
                |mseq_ctx, conductor, controller| {
                    mseq_ctx.handle_inputs(conductor, controller, &mut inputs)
                },
            );
        }
    }

    #[task(priority = 1, local = [display], shared = [display_text])]
    async fn update_display(mut cx: update_display::Context) {
        cx.shared
            .display_text
            .lock(|display_text| cx.local.display.update(display_text));
    }
}
