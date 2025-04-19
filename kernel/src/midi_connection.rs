use crate::driver::{write, DriverError};
use stm32f4xx_hal::{block, pac::USART1, serial::Tx};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MidiError {
    #[error("MIDI: Error when calling Diver.\n\t`{0}`")]
    Util(#[from] DriverError),
}

pub struct MidiOut {
    tx: Tx<USART1>,
}

impl MidiOut {
    pub fn new(tx: Tx<USART1>) -> Self {
        Self { tx }
    }
}

const CLOCK: u8 = 0xf8;
const START: u8 = 0xfa;
const CONTINUE: u8 = 0xfb;
const STOP: u8 = 0xfc;
const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
const CC: u8 = 0xB0;

impl mseq::MidiOut for MidiOut {
    type Error = MidiError;
    fn send_start(&mut self) -> Result<(), MidiError> {
        Ok(write(&mut self.tx, &[START])?)
    }
    fn send_continue(&mut self) -> Result<(), MidiError> {
        Ok(write(&mut self.tx, &[CONTINUE])?)
    }
    fn send_stop(&mut self) -> Result<(), MidiError> {
        Ok(write(&mut self.tx, &[STOP])?)
    }
    fn send_clock(&mut self) -> Result<(), MidiError> {
        Ok(write(&mut self.tx, &[CLOCK])?)
    }
    fn send_note_on(&mut self, channel_id: u8, note: u8, velocity: u8) -> Result<(), MidiError> {
        Ok(write(
            &mut self.tx,
            &[NOTE_ON | (channel_id - 1), note, velocity],
        )?)
    }
    fn send_note_off(&mut self, channel_id: u8, note: u8) -> Result<(), MidiError> {
        Ok(write(
            &mut self.tx,
            &[NOTE_OFF | (channel_id - 1), note, 0],
        )?)
    }
    fn send_cc(&mut self, channel_id: u8, parameter: u8, value: u8) -> Result<(), MidiError> {
        Ok(write(
            &mut self.tx,
            &[CC | (channel_id - 1), parameter, value],
        )?)
    }
}
