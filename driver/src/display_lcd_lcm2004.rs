use log::{debug, error, info};
use stm32f4xx_hal::{
    pac::{I2C1, TIM3},
    timer::DelayUs,
};

const LCD_ADDRESS: u8 = 0x27;

#[derive(Default)]
pub struct DisplayText {
    pub lines: [heapless::String<16>; 4],
}

pub struct Lcd {
    i2c: stm32f4xx_hal::i2c::I2c<I2C1>,
    delay: DelayUs<TIM3>,
    current_display: DisplayText,
}

impl Lcd {
    pub fn new(i2c: stm32f4xx_hal::i2c::I2c<I2C1>, delay: DelayUs<TIM3>) -> Option<Self> {
        let mut result = Self {
            i2c,
            delay,
            current_display: DisplayText::default(),
        };
        match result.init() {
            Some(mut lcd) => {
                info!("Screen detected");
                lcd.clear().unwrap();
                lcd.return_home().unwrap()
            }
            None => {
                error!("Screen initialization failed");
                return None;
            }
        }
        Some(result)
    }

    fn update_cursor_pos(
        lcd: &mut lcd_lcm1602_i2c::sync_lcd::Lcd<
            '_,
            stm32f4xx_hal::i2c::I2c<I2C1>,
            stm32f4xx_hal::timer::Delay<stm32f4xx_hal::pac::TIM3, 1000000>,
        >,
        row: usize,
        col: usize,
        cursor_row: &mut u8,
        cursor_col: &mut u8,
    ) {
        let display_row = (row % 2) as u8;
        let display_col = (col + (row / 2) * 20) as u8;
        if display_row != *cursor_row || display_col != *cursor_col {
            lcd.set_cursor(display_row, display_col).unwrap();
            *cursor_col = display_col;
            *cursor_row = display_row;
        }
    }

    pub fn update(&mut self, text: &DisplayText) {
        let mut lcd = lcd_lcm1602_i2c::sync_lcd::Lcd::new(&mut self.i2c, &mut self.delay)
            .with_address(LCD_ADDRESS)
            .with_rows(2);
        lcd.return_home().unwrap();
        let mut cursor_row = 0;
        let mut cursor_col = 0;
        text.lines
            .iter()
            .zip(self.current_display.lines.iter())
            .enumerate()
            .for_each(|(row, (new, old))| {
                new.chars()
                    .zip(old.chars())
                    .enumerate()
                    .for_each(|(col, (new_c, old_c))| {
                        if new_c != old_c {
                            Self::update_cursor_pos(
                                &mut lcd,
                                row,
                                col,
                                &mut cursor_row,
                                &mut cursor_col,
                            );
                            lcd.write_str(&(new.as_str())[col..(col + 1)]).unwrap();
                            cursor_col += 1;
                        }
                    });
                if new.len() > old.len() {
                    let col = old.len();
                    Self::update_cursor_pos(&mut lcd, row, col, &mut cursor_row, &mut cursor_col);
                    lcd.write_str(&(new.as_str())[col..]).unwrap();
                }
            });
    }

    fn init(
        &mut self,
    ) -> Option<
        lcd_lcm1602_i2c::sync_lcd::Lcd<
            '_,
            stm32f4xx_hal::i2c::I2c<I2C1>,
            stm32f4xx_hal::timer::Delay<stm32f4xx_hal::pac::TIM3, 1000000>,
        >,
    > {
        lcd_lcm1602_i2c::sync_lcd::Lcd::new(&mut self.i2c, &mut self.delay)
            .with_address(LCD_ADDRESS)
            .with_rows(2)
            .with_cursor_on(false)
            .init()
            .ok()
    }
}
