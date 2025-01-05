#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use mseq_hardware as _; // global logger + panicking-behavior + memory layout

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

    use rtic_monotonics::systick::prelude::*;
    use stm32f4xx_hal::{
        dma::{self, DmaEvent, Transfer},
        pac::{DMA2, I2C1, TIM3, USART1},
        prelude::*,
        serial::{
            config::{DmaConfig, StopBits::STOP1},
            Config, Serial,
        },
        timer::{self, CounterHz, DelayUs},
        ClearFlags,
    };

    systick_monotonic!(Mono, 100);

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
        timer: CounterHz<stm32f4xx_hal::pac::TIM2>,
        lcd: Lcd,
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

        // Fixed time update
        let mut timer = cx.device.TIM2.counter_hz(&clocks);
        timer.listen(timer::Event::Update);
        timer.start(1.Hz()).unwrap();

        // lcd screen
        let gpiob = cx.device.GPIOB.split();
        let mut i2c = stm32f4xx_hal::i2c::I2c::new(
            cx.device.I2C1,
            (gpiob.pb6, gpiob.pb7),
            stm32f4xx_hal::i2c::Mode::standard(50.kHz()),
            &clocks,
        );
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

        (
            Shared { transfer },
            Local {
                buffer: Some(cx.local.buf2),
                timer,
                lcd: Lcd {
                    i2c,
                    delay,
                },
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

    #[task(binds = TIM2, priority = 2, local = [timer, lcd])]
    fn fixed_time(cx: fixed_time::Context) {
        // Fixed time update
        // defmt::info!("tick");
        cx.local.timer.clear_all_flags();
        let mut lcd = cx.local.lcd.get();

        lcd.set_cursor(0, 20).unwrap();
        lcd.write_str("test3").unwrap();

        lcd.set_cursor(1, 20).unwrap();
        lcd.write_str("test4").unwrap();
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
            "dma buffer full: {}, {}, {}, {}, {}, {}, {}, {}",
            buffer[0],
            buffer[1],
            buffer[2],
            buffer[3],
            buffer[4],
            buffer[5],
            buffer[6],
            buffer[7],
        );
        buffer.fill(0);
        *local.buffer = Some(buffer);
    }
}
