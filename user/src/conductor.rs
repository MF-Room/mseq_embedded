extern crate alloc;
use alloc::{vec, vec::Vec};
use log::trace;
use mseq_core::{Conductor, DeteTrack, Instruction, MidiNote, Track};
use postcard::from_bytes;

struct MyTrack {
    channel_id: u8,
}
const ACID_TRACK: &[u8] = include_bytes!("../../res/test.bin");

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

            vec![Instruction::PlayNote {
                midi_note: note,
                len: note_length,
                channel_id: self.channel_id,
            }]
            //midi_controller.play_note(note, note_length, self.channel_id);
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
        let mut notes = self.track.play_step(step);
        notes.extend(self.acid.play_step(step));
        notes
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
