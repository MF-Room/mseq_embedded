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
    dispatchers = [ADC, DMA1_STREAM0, DMA2_STREAM5],
    peripherals = true,
)]

mod app {
    use log::{debug, error, info, trace, warn};
    use mseq_core::MidiMessage;
    use mseq_core::*;
    use rtic::mutex_prelude::TupleExt02;
    use rtic::mutex_prelude::TupleExt03;
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

    use crate::app::shared_resources::*;
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
        display: Option<driver::Lcd>,
        is_master: bool,
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
        let pa1 = gpioa.pa1.into_floating_input();
        let is_master = pa1.is_high();
        if pa1.is_high() {
            info!("Master mode");
        } else {
            info!("Slave mode");
        }

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
        let (tx, mut rx) = serial.split();
        rx.listen();

        // lcd screen
        /*
        let gpiob = cx.device.GPIOB.split();
        let i2c = stm32f4xx_hal::i2c::I2c::new(
            cx.device.I2C1,
            (gpiob.pb6, gpiob.pb7),
            stm32f4xx_hal::i2c::Mode::standard(50.kHz()),
            &clocks,
        );
        let delay = cx.device.TIM3.delay_us(&clocks);
        let display = driver::Lcd::new(i2c, delay);
        */
        let display = None;

        // MidiOut
        let midi_out = MidiOut::new(tx);

        let mut conductor = conductor::UserConductor::default();
        let mut midi_controller = MidiController::new(midi_out);
        let mut mseq_ctx = mseq_core::Context::default();

        // Clock
        let mut rtc = Rtc::new(cx.device.RTC, &mut cx.device.PWR);
        let clock_period = mseq_ctx.get_period_us() as u32;
        if is_master {
            rtc.enable_wakeup(clock_period.micros::<1, 1_000_000>().into());
            rtc.listen(&mut cx.device.EXTI, stm32f4xx_hal::rtc::Event::Wakeup);
        }

        // Input Queue
        let input_queue = InputQueue::new();

        // Input Signal
        let (w, r) = make_signal!(());
        handle_input::spawn(r).unwrap();

        // Conductor Init
        mseq_ctx.init(&mut conductor, &mut midi_controller);

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
                is_master,
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
    fn master_clock(mut cx: master_clock::Context) {
        // Clear clock interrupt flag
        cx.local
            .rtc
            .clear_interrupt(stm32f4xx_hal::rtc::Event::Wakeup);

        clock(
            &mut cx.shared.mseq_ctx,
            &mut cx.shared.midi_controller,
            &mut cx.shared.conductor,
            &mut cx.shared.display_text,
        );

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

    fn clock(
        mut mseq_ctx: &mut mseq_ctx_that_needs_to_be_locked,
        mut midi_controller: &mut midi_controller_that_needs_to_be_locked,
        mut conductor: &mut conductor_that_needs_to_be_locked,
        mut display_text: &mut display_text_that_needs_to_be_locked,
    ) {
        trace!("Clock");

        // mseq logic
        // post tick
        (&mut mseq_ctx, &mut midi_controller)
            .lock(|mseq_ctx, midi_controller| mseq_ctx.process_post_tick(midi_controller));

        let mut current_step = 0;
        // pre tick
        (&mut mseq_ctx, &mut midi_controller, &mut conductor).lock(
            |mseq_ctx, midi_controller, conductor| {
                mseq_ctx.process_pre_tick(conductor, midi_controller);
                current_step = mseq_ctx.get_step();
            },
        );

        // Screen is refreshed on each beat
        if current_step % 24 == 1 {
            // Update display text
            (&mut conductor, &mut mseq_ctx, &mut display_text).lock(
                |conductor, ctx, display_text| {
                    *display_text = conductor.display_text(ctx);
                },
            );
            match update_display::spawn() {
                Ok(_) => (),
                Err(_) => warn!("Display update skipped"),
            }
        }
    }

    #[task(priority = 3, shared = [conductor, midi_controller, mseq_ctx, display_text])]
    async fn slave_clock(mut cx: slave_clock::Context) {
        clock(
            &mut cx.shared.mseq_ctx,
            &mut cx.shared.midi_controller,
            &mut cx.shared.conductor,
            &mut cx.shared.display_text,
        );
    }

    #[task(priority = 3, shared = [mseq_ctx])]
    async fn slave_start(mut cx: slave_start::Context) {
        cx.shared.mseq_ctx.lock(|ctx| ctx.start());
    }

    #[task(priority = 3, shared = [mseq_ctx])]
    async fn slave_stop(mut cx: slave_stop::Context) {
        cx.shared.mseq_ctx.lock(|ctx| ctx.pause());
    }

    #[task(priority = 3, shared = [mseq_ctx])]
    async fn slave_continue(mut cx: slave_continue::Context) {
        cx.shared.mseq_ctx.lock(|ctx| ctx.resume());
    }

    // Midi interrupt
    #[task(binds = USART1, priority = 4, local=[rx, midi_input_handler, input_signal_writer, is_master], shared = [input_queue])]
    fn midi_int(mut cx: midi_int::Context) {
        let serial = cx.local.rx;
        match serial.read() {
            Ok(b) => {
                debug!("{b} received");
                cx.local
                    .midi_input_handler
                    .process_byte(b)
                    .map(|midi_message| {
                        match midi_message {
                            MidiMessage::Clock => {
                                if !*cx.local.is_master {
                                    if let Err(()) = slave_clock::spawn() {
                                        error!("Clock cycle skipped")
                                    }
                                } else {
                                    warn!("Received clock signal but mode is set to master")
                                }
                            }
                            MidiMessage::Start => {
                                if !*cx.local.is_master {
                                    if let Err(()) = slave_start::spawn() {
                                        error!("Failed to start sequencer")
                                    }
                                } else {
                                    warn!("Received start signal but mode is set to master")
                                }
                            }
                            MidiMessage::Stop => {
                                if !*cx.local.is_master {
                                    if let Err(()) = slave_stop::spawn() {
                                        error!("Failed to stop sequencer")
                                    }
                                } else {
                                    warn!("Received stop signal but mode is set to master")
                                }
                            }
                            MidiMessage::Continue => {
                                if !*cx.local.is_master {
                                    if let Err(()) = slave_continue::spawn() {
                                        error!("Failed to continue sequencer")
                                    }
                                } else {
                                    warn!("Received continue signal but mode is set to master")
                                }
                            }
                            _ => {
                                cx.shared
                                    .input_queue
                                    .lock(|input_queue| input_queue.push_back(midi_message));
                                cx.local.input_signal_writer.write(());
                            }
                        };
                    });
            }
            Err(_) => error!("Serial error"),
        }
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
                    mseq_ctx.handle_input(conductor, controller, &mut inputs)
                },
            );
        }
    }

    #[task(priority = 1, local = [display], shared = [display_text])]
    async fn update_display(mut cx: update_display::Context) {
        if let Some(display) = cx.local.display.as_mut() {
            cx.shared
                .display_text
                .lock(|display_text| display.update(display_text))
        }
    }
}
