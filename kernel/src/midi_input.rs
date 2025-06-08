use mseq_core::{MidiMessage, MidiNote};

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
    pub fn process_byte(&mut self, byte: u8) -> Option<(u8, MidiMessage)> {
        // Handle real-time messages (e.g., Clock) immediately
        if byte == 0xF8 {
            return Some((0, MidiMessage::Clock));
        }

        // Append the byte to the buffer
        self.push(byte);

        // Must start with a status byte
        if self.data.len() == 1 && self.data[0] & 0x80 == 0 {
            // First byte is not a status byte; discard
            self.clear();
            return None;
        }

        if self.data.is_empty() {
            return None;
        }

        let status = self.data[0];
        let message_type = status & 0xF0;
        let channel = (status & 0x0F) + 1;

        // If we have a status byte and 2 data bytes, process the message
        if self.size < 3 {
            return None;
        }

        let msg = match message_type {
            0x80 => {
                let key = self.data[1];
                let vel = self.data[2];
                let note = MidiNote::from_midi_value(key, vel);
                Some((channel, MidiMessage::NoteOff { note }))
            }
            0x90 => {
                let key = self.data[1];
                let vel = self.data[2];
                let note = MidiNote::from_midi_value(key, vel);
                Some((channel, MidiMessage::NoteOn { note }))
            }
            0xB0 => {
                let controller = self.data[1];
                let value = self.data[2];
                Some((channel, MidiMessage::CC { controller, value }))
            }
            _ => None,
        };

        // Reset buffer
        self.clear();
        msg
    }
}
