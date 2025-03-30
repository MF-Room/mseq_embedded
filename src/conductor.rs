pub struct Conductor {}

impl Conductor {
    pub fn new() -> Self {
        Conductor {}
    }
}

impl mseq::Conductor for Conductor {
    fn init(&mut self, context: &mut mseq::Context<impl mseq::MidiOut>) {}

    fn update(&mut self, context: &mut mseq::Context<impl mseq::MidiOut>) {}
}
