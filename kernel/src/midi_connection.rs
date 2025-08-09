use driver::{DriverError, write};
use log::debug;
use mseq_core::MidiNote;
use stm32f4xx_hal::{pac::USART1, serial::Tx};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MidiError {
    #[error("Error when calling Diver.\n\tDriver: {0}")]
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

pub const CLOCK: u8 = 0xf8;
pub const START: u8 = 0xfa;
pub const CONTINUE: u8 = 0xfb;
pub const STOP: u8 = 0xfc;
pub const NOTE_ON: u8 = 0x90;
pub const NOTE_OFF: u8 = 0x80;
pub const CC: u8 = 0xB0;
pub const PC: u8 = 0xC0;

impl mseq_core::MidiOut for MidiOut {
    type Error = MidiError;
    fn send_start(&mut self) -> Result<(), MidiError> {
        debug!("Send Start");
        Ok(write(&mut self.tx, &[START])?)
    }
    fn send_continue(&mut self) -> Result<(), MidiError> {
        debug!("Send Continue");
        Ok(write(&mut self.tx, &[CONTINUE])?)
    }
    fn send_stop(&mut self) -> Result<(), MidiError> {
        debug!("Send Stop");
        Ok(write(&mut self.tx, &[STOP])?)
    }
    fn send_clock(&mut self) -> Result<(), MidiError> {
        debug!("Send Clock");
        Ok(write(&mut self.tx, &[CLOCK])?)
    }
    fn send_note_on(&mut self, channel_id: u8, note: u8, velocity: u8) -> Result<(), MidiError> {
        debug!(
            "Send Note On: Channel: {channel_id}, Note: {:?}",
            MidiNote::from_midi_value(note, velocity)
        );
        Ok(write(
            &mut self.tx,
            &[NOTE_ON | (channel_id - 1), note, velocity],
        )?)
    }
    fn send_note_off(&mut self, channel_id: u8, note: u8) -> Result<(), MidiError> {
        debug!(
            "Send Note Off: Channel: {channel_id}, Note: {:?}",
            MidiNote::from_midi_value(note, 0)
        );
        Ok(write(
            &mut self.tx,
            &[NOTE_OFF | (channel_id - 1), note, 0],
        )?)
    }
    fn send_cc(&mut self, channel_id: u8, parameter: u8, value: u8) -> Result<(), MidiError> {
        debug!("Send CC: Channel: {channel_id}, paramerte: {parameter}, value: {value}");
        Ok(write(
            &mut self.tx,
            &[CC | (channel_id - 1), parameter, value],
        )?)
    }
    fn send_pc(&mut self, channel_id: u8, value: u8) -> Result<(), MidiError> {
        debug!("Send PC: Channel: {channel_id}, value: {value}");
        Ok(write(&mut self.tx, &[PC | (channel_id - 1), value])?)
    }
}
