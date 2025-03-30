use mseq::{self, MidiError};
use stm32f4xx_hal::{pac::USART1, prelude::*, serial::Tx};

pub struct MidiOut {
    tx: Tx<USART1>,
}

impl MidiOut {
    pub fn new(tx: Tx<USART1>) -> Self {
        Self { tx }
    }
}

impl mseq::MidiOut for MidiOut {
    fn send_start(&mut self) -> Result<(), MidiError> {
        //TODO
        Ok(())
    }
    fn send_continue(&mut self) -> Result<(), MidiError> {
        //TODO
        Ok(())
    }
    fn send_stop(&mut self) -> Result<(), MidiError> {
        //TODO
        Ok(())
    }
    fn send_clock(&mut self) -> Result<(), MidiError> {
        match self.tx.write(0xf8) {
            Ok(_) => Ok(()),
            Err(_) => {
                //TODO
                // defmt::info!("send error");
                Ok(())
            }
        }
    }

    fn send_note_on(&mut self, channel_id: u8, note: u8, velocity: u8) -> Result<(), MidiError> {
        //TODO
        Ok(())
    }
    fn send_note_off(&mut self, channel_id: u8, note: u8) -> Result<(), MidiError> {
        //TODO
        Ok(())
    }
    fn send_cc(&mut self, channel_id: u8, parameter: u8, value: u8) -> Result<(), MidiError> {
        //TODO
        Ok(())
    }
}
