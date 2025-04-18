use log::trace;
use mseq::{Conductor, MidiNote, Track};

struct MyTrack {
    channel_id: u8,
}

// Implement a track for full freedom (randomization, automatization...)
impl Track for MyTrack {
    fn play_step(
        &mut self,
        step: u32,
        midi_controller: &mut mseq::MidiController<impl mseq::MidiOut>,
    ) {
        // Midi channel id to send the note to
        if step % 24 == 0 {
            trace!("Play track {}", step);

            // Choose a random note
            let note = MidiNote {
                note: mseq::Note::C,
                octave: 4,
                vel: 127,
            };

            // Note length in number of steps
            let note_length = 12;

            // Request to play the note to the midi controller.
            midi_controller.play_note(note, note_length, self.channel_id);
        }
    }
}

pub struct UserConductor {
    track: MyTrack,
}

impl Conductor for UserConductor {
    fn init(&mut self, context: &mut mseq::Context<impl mseq::MidiOut>) {
        // The sequencer is on pause by default
        context.start();
    }

    fn update(&mut self, context: &mut mseq::Context<impl mseq::MidiOut>) {
        // The conductor plays the track
        context.midi.play_track(&mut self.track);

        // Quit after 960 steps
        if context.get_step() == 959 {
            context.quit();
        }
    }
}

impl UserConductor {
    pub fn new() -> Self {
        Self {
            track: MyTrack { channel_id: 10 },
        }
    }
}
