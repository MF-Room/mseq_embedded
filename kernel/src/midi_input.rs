use mseq_core::{MidiMessage, MidiNote};

use crate::midi_connection::{CC, CLOCK, CONTINUE, NOTE_OFF, NOTE_ON, PC, START, STOP};

pub struct MidiInputHandler {
    size: u8,
    data: [u8; 3],
}

impl MidiInputHandler {
    pub fn new() -> Self {
        Self {
            size: 0,
            data: [0; 3],
        }
    }
    fn push(&mut self, data: u8) {
        self.data[self.size as usize] = data;
        self.size += 1;
    }
    fn clear(&mut self) {
        self.size = 0;
    }
    pub fn process_byte(&mut self, byte: u8) -> Option<MidiMessage> {
        // Handle real-time messages immediately
        match byte {
            CLOCK => return Some(MidiMessage::Clock),
            START => return Some(MidiMessage::Start),
            STOP => return Some(MidiMessage::Stop),
            CONTINUE => return Some(MidiMessage::Continue),
            _ => {}
        }
        // Append the byte to the buffer
        self.push(byte);

        // Must start with a known status byte
        if self.data.len() == 1 && ![CC, PC, NOTE_ON, NOTE_OFF].contains(&self.data[0]) {
            // First byte is not a status byte; discard
            self.clear();
            return None;
        }

        // If we have a single status byte
        if self.size == 1 {
            return None;
        }

        let status = self.data[0];
        let message_type = status & 0xF0;
        let channel = (status & 0x0F) + 1;

        // If we have 2 bytes
        if self.size == 2 {
            if message_type == PC {
                let value = self.data[1];
                // Reset buffer
                self.clear();
                return Some(MidiMessage::PC { channel, value });
            } else {
                return None;
            }
        }

        let msg = match message_type {
            NOTE_OFF => {
                let key = self.data[1];
                let vel = self.data[2];
                let note = MidiNote::from_midi_value(key, vel);
                Some(MidiMessage::NoteOff { channel, note })
            }
            NOTE_ON => {
                let key = self.data[1];
                let vel = self.data[2];
                let note = MidiNote::from_midi_value(key, vel);
                Some(MidiMessage::NoteOn { channel, note })
            }
            CC => {
                let controller = self.data[1];
                let value = self.data[2];
                Some(MidiMessage::CC {
                    channel,
                    controller,
                    value,
                })
            }
            _ => None,
        };

        // Reset buffer
        self.clear();
        msg
    }
}
