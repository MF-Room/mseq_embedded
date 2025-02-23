#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

extern crate alloc;
mod debug;
mod midi_connection;

#[rtic::app(
    device = stm32f4xx_hal::pac,
    // TODO: Replace the `FreeInterrupt1, ...` with free interrupt vectors if software tasks are used
    // You can usually find the names of the interrupt vectors in the some_hal::pac::interrupt enum.
    dispatchers = [ADC],
    peripherals = true,
)]

mod app {
    const BUFFER_SIZE: usize = 8;
    const LCD_ADDRESS: u8 = 0x27;

    use crate::midi_connection::MidiOut;
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_alloc::LlffHeap as Heap;
    use mseq::MidiController;

    #[global_allocator]
    static HEAP: Heap = Heap::empty();

    use rtic_monotonics::systick::prelude::*;
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

    //TODO: understand and add comment
    systick_monotonic!(Mono, 100);

    /*
    struct Lcd {
        i2c: stm32f4xx_hal::i2c::I2c<I2C1>,
        delay: DelayUs<TIM3>,
    }

    impl Lcd {
        fn get(
            &mut self,
        ) -> lcd_lcm1602_i2c::sync_lcd::Lcd<
            '_,
            stm32f4xx_hal::i2c::I2c<I2C1>,
            stm32f4xx_hal::timer::Delay<stm32f4xx_hal::pac::TIM3, 1000000>,
        > {
            lcd_lcm1602_i2c::sync_lcd::Lcd::new(&mut self.i2c, &mut self.delay)
                .with_address(LCD_ADDRESS)
                .with_rows(2)
        }
    }
    */

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        // lcd: Lcd,
        rx: Rx<USART1>,
        counter: u32,
        rtc: Rtc,
        midi_controller: MidiController<MidiOut>,
    }

    #[init(local = [buf1: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE], buf2: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE]])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");

        // Initilialize allocator
        allocator_init();

        // Serial connection
        let rcc = cx.device.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).freeze();
        let gpioa = cx.device.GPIOA.split();
        let rx_1 = gpioa.pa10.into_alternate();
        let tx_1 = gpioa.pa9.into_alternate();

        defmt::info!("init");

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

        defmt::info!("init");
        tx.write(0xfa).unwrap();

        // Clock
        let mut rtc = Rtc::new(cx.device.RTC, &mut cx.device.PWR);
        rtc.enable_wakeup(17606.micros::<1, 1_000_000>().into());
        rtc.listen(&mut cx.device.EXTI, stm32f4xx_hal::rtc::Event::Wakeup);

        /*
        // lcd screen
        let gpiob = cx.device.GPIOB.split();
        let mut i2c = stm32f4xx_hal::i2c::I2c::new(
            cx.device.I2C1,
            (gpiob.pb6, gpiob.pb7),
            stm32f4xx_hal::i2c::Mode::standard(50.kHz()),
            &clocks,
        );

        defmt::info!("init");
        let mut delay = cx.device.TIM3.delay_us(&clocks);
        {
            let mut lcd = lcd_lcm1602_i2c::sync_lcd::Lcd::new(&mut i2c, &mut delay)
                .with_address(LCD_ADDRESS)
                .with_rows(2)
                .with_cursor_on(true)
                .init()
                .unwrap();

            lcd.return_home().unwrap();
            lcd.clear().unwrap();
            lcd.write_str("test").unwrap();
            lcd.set_cursor(1, 0).unwrap();
            lcd.write_str("test2").unwrap();
        }
        */

        // Test allocator
        let mut v: Vec<u32> = vec![];
        v.push(1);

        // MidiOut
        let midi_out = MidiOut::new(tx);
        defmt::info!("init over!");

        (
            Shared {},
            Local {
                //lcd: Lcd { i2c, delay },
                rx,
                counter: 0,
                rtc,
                midi_controller: MidiController::new(midi_out),
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

    #[task(binds = RTC_WKUP, priority = 2, local = [counter, rtc, midi_controller])]
    fn clock(cx: clock::Context) {
        cx.local
            .rtc
            .clear_interrupt(stm32f4xx_hal::rtc::Event::Wakeup);

        cx.local.midi_controller.send_clock();

        /*
        match cx.local.tx.write(0xf8) {
            Ok(_) => {}
            Err(_) => defmt::info!("send error"),
        }
        */

        if *cx.local.counter % 24 == 0 {
            defmt::info!("tick");
        }

        *cx.local.counter += 1;
    }

    // Midi interrupt
    #[task(binds = USART1, priority = 2, local=[rx])]
    fn midi_int(cx: midi_int::Context) {
        let serial = cx.local.rx;
        match serial.read() {
            Ok(b) => defmt::info!("Received: {}", b),
            Err(_) => defmt::info!("Serial is empty"),
        }
    }

    fn allocator_init() {
        const HEAP_SIZE: usize = 1024;
        use core::mem::MaybeUninit;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }
}
