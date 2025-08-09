use alloc::vec::Vec;
use alloc::{format, vec};
use log::{debug, trace};
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
    fn init(&mut self, context: &mut mseq_core::Context) -> Vec<Instruction> {
        // The sequencer is on pause by default
        trace!("Initializing conductor");
        vec![context.start()]
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
        instructions
    }

    fn handle_input(
        &mut self,
        input: mseq_core::MidiMessage,
        _context: &Context,
    ) -> Vec<Instruction> {
        match input {
            mseq_core::MidiMessage::NoteOff { channel, note } => {
                vec![Instruction::MidiMessage {
                    midi_message: MidiMessage::NoteOff {
                        channel,
                        note: note.transpose(3),
                    },
                }]
            }
            mseq_core::MidiMessage::NoteOn { channel, note } => {
                vec![Instruction::MidiMessage {
                    midi_message: MidiMessage::NoteOn {
                        channel,
                        note: note.transpose(3),
                    },
                }]
            }
            _ => vec![],
        }
    }
}

impl Default for UserConductor {
    fn default() -> Self {
        let c = Self {
            acid: from_bytes(ACID_TRACK).unwrap(),
            track: MyTrack { channel_id: 1 },
        };
        //trace!("{:?}", c.acid);
        c
    }
}

impl UserConductor {
    pub fn display_text(&self, context: &Context) -> driver::DisplayText {
        let line0 = heapless::String::try_from(" -- Mseq -- ").unwrap();
        let line1 =
            heapless::String::try_from(format!("Bpm: {}", context.get_bpm()).as_str()).unwrap();
        let line2 =
            heapless::String::try_from(format!("Step: {}", context.get_step() / 24).as_str())
                .unwrap();
        let line3 = heapless::String::try_from("Machines play").unwrap();

        driver::DisplayText {
            lines: [line0, line1, line2, line3],
        }
    }
}
