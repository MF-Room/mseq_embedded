use stm32f4xx_hal::{
    pac::{I2C1, TIM3},
    timer::DelayUs,
};

const LCD_ADDRESS: u8 = 0x27;

pub struct Lcd {
    i2c: stm32f4xx_hal::i2c::I2c<I2C1>,
    delay: DelayUs<TIM3>,
}

impl Lcd {
    pub fn new(i2c: stm32f4xx_hal::i2c::I2c<I2C1>, delay: DelayUs<TIM3>) -> Self {
        Self { i2c, delay }
    }

    pub fn get(
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
