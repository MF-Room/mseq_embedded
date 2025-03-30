#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

extern crate alloc;
mod conductor;
mod exit;
mod heap;
mod midi_connection;
mod screen;

use crate::conductor::*;

use panic_rtt_target as _;

#[rtic::app(
    device = stm32f4xx_hal::pac,
    // TODO: Replace the `FreeInterrupt1, ...` with free interrupt vectors if software tasks are used
    // You can usually find the names of the interrupt vectors in the some_hal::pac::interrupt enum.
    dispatchers = [ADC],
    peripherals = true,
)]

mod app {
    const BUFFER_SIZE: usize = 8;

    use alloc::vec;
    use alloc::vec::Vec;
    use mseq::Conductor;
    use mseq::MidiController;
    use rtic_monotonics::systick::prelude::*;
    use rtt_target::{rprintln, rtt_init_print};
    use stm32f4xx_hal::{
        pac::{I2C1, TIM3, USART1},
        prelude::*,
        rtc::Rtc,
        serial::{
            config::{DmaConfig, StopBits::STOP1},
            Config, Rx, Serial, Tx,
        },
        timer::DelayUs,
    };

    use crate::conductor;
    use crate::heap;
    use crate::midi_connection::MidiOut;
    use crate::screen;

    //TODO: understand and add comment
    systick_monotonic!(Mono, 100);

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        lcd: screen::Lcd,
        rx: Rx<USART1>,
        counter: u32,
        rtc: Rtc,
        midi_controller: MidiController<MidiOut>,
        conductor: conductor::Conductor,
    }

    #[init(local = [buf1: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE], buf2: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE]])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!("Init");
        // defmt::info!("init");

        // Initilialize allocator
        heap::allocator_init();

        // Serial connection
        let rcc = cx.device.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).freeze();
        let gpioa = cx.device.GPIOA.split();
        let rx_1 = gpioa.pa10.into_alternate();
        let tx_1 = gpioa.pa9.into_alternate();

        //        defmt::info!("init");

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

        //        defmt::info!("init");
        tx.write(0xfa).unwrap();

        // Clock
        let mut rtc = Rtc::new(cx.device.RTC, &mut cx.device.PWR);
        rtc.enable_wakeup(17606.micros::<1, 1_000_000>().into());
        rtc.listen(&mut cx.device.EXTI, stm32f4xx_hal::rtc::Event::Wakeup);

        // lcd screen
        let gpiob = cx.device.GPIOB.split();
        let mut i2c = stm32f4xx_hal::i2c::I2c::new(
            cx.device.I2C1,
            (gpiob.pb6, gpiob.pb7),
            stm32f4xx_hal::i2c::Mode::standard(50.kHz()),
            &clocks,
        );

        // defmt::info!("init");
        let mut delay = cx.device.TIM3.delay_us(&clocks);

        // Test allocator
        let mut v: Vec<u32> = vec![];
        v.push(1);

        // MidiOut
        let midi_out = MidiOut::new(tx);
        //       defmt::info!("init over!");

        let conductor = conductor::Conductor::new();

        let mut midi_controller = MidiController::new(midi_out);
        midi_controller.start();

        (
            Shared {},
            Local {
                lcd: screen::Lcd::new(i2c, delay),
                rx,
                counter: 0,
                rtc,
                midi_controller,
                conductor,
            },
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        //        defmt::info!("idle");

        loop {
            continue;
        }
    }

    #[task(binds = RTC_WKUP, priority = 2, local = [counter, rtc, midi_controller])]
    fn clock(cx: clock::Context) {
        cx.local
            .rtc
            .clear_interrupt(stm32f4xx_hal::rtc::Event::Wakeup);

        cx.local.midi_controller.send_clock();

        if *cx.local.counter % 24 == 0 {
            //            defmt::info!("tick");
        }

        *cx.local.counter += 1;
    }

    // Midi interrupt
    #[task(binds = USART1, priority = 2, local=[rx])]
    fn midi_int(cx: midi_int::Context) {
        let serial = cx.local.rx;
        match serial.read() {
            Ok(b) => {}  // defmt::info!("Received: {}", b),
            Err(_) => {} //defmt::info!("Serial is empty"),
        }
    }
}
