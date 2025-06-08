use alloc::vec;
use alloc::vec::Vec;
use log::trace;
use mseq_core::*;
use postcard::from_bytes;

struct MyTrack {
    channel_id: u8,
}
const ACID_TRACK: &[u8] = include_bytes!("../../track_bin/acid.bin");

// Implement a track for full freedom (randomization, automatization...)
impl Track for MyTrack {
    fn play_step(&mut self, step: u32) -> Vec<Instruction> {
        // Midi channel id to send the note to
        if step % 24 == 0 {
            trace!("Play track {}", step);

            let note = MidiNote {
                note: mseq_core::Note::C,
                octave: 4,
                vel: 127,
            };

            // Note length in number of steps
            let note_length = 12;

            // Request to play the note to the midi controller.
            vec![Instruction::PlayNote {
                midi_note: note,
                len: note_length,
                channel_id: self.channel_id,
            }]
        } else {
            vec![]
        }
    }
}

pub struct UserConductor {
    track: MyTrack,
    acid: DeteTrack,
}

impl Conductor for UserConductor {
    fn init(&mut self, context: &mut mseq_core::Context) {
        // The sequencer is on pause by default
        context.start();
    }

    fn update(&mut self, context: &mut mseq_core::Context) -> Vec<Instruction> {
        let step = context.get_step();

        // Quit after 960 steps
        if step == 959 {
            context.quit();
            return vec![];
        }

        // The conductor plays the track
        let mut instructions = self.track.play_step(step);
        instructions.extend(self.acid.play_step(step));

        // instructions
        vec![]
    }

    fn handle_input(
        &mut self,
        channel_id: u8,
        input: MidiMessage,
        _context: &Context,
    ) -> Vec<Instruction> {
        vec![match input {
            MidiMessage::NoteOff { note } => Instruction::MidiMessage {
                channel_id,
                midi_message: MidiMessage::NoteOff {
                    note: note.transpose(3),
                },
            },
            MidiMessage::NoteOn { note } => Instruction::MidiMessage {
                channel_id,
                midi_message: MidiMessage::NoteOn {
                    note: note.transpose(3),
                },
            },
            _ => Instruction::MidiMessage {
                channel_id,
                midi_message: input,
            },
        }]
    }
}

impl UserConductor {
    pub fn new() -> Self {
        let c = Self {
            acid: from_bytes(ACID_TRACK).unwrap(),
            track: MyTrack { channel_id: 1 },
        };
        trace!("{:?}", c.acid);
        c
    }
}
