use std::{sync::mpsc, thread::sleep};

use chrono::Duration;
use pitch_calc::*;
use timer::Timer;

use midir::MidiOutputConnection;

use crate::pitch::*;
use crate::trigger::*;

//constants
const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;
const PROGRAM_CHANGE_MSG: u8 = 0xC0;
const VELOCITY: u8 = 0x64;

pub struct SequencerConfiguration {
    pub instrument: u8,
    pub quantizer_scale: Vec<Letter>,
    pub bpm: f32, // beats per minutes
}

enum SequencerCommand {
    Start,
    Stop,
    SetPitchProducer(Box<dyn PitchModule>),
    SetTriggerProducer(Box<dyn TriggerModule>),
    SetInstrument(u8),
}

pub struct Sequencer {
    sender: mpsc::Sender<SequencerCommand>,
    _timer: Timer,
}

impl Sequencer {
    pub fn new(config: SequencerConfiguration, is_playing: bool) -> Sequencer {
        // Create async communication channel to the sequencer thread
        let (tx, rx) = mpsc::channel();
        let mut thread = SequencerThread::new(
            rx,
            Sequencer::build_pitch_producer(&config),
            Sequencer::build_trigger_producer(&config),
            is_playing,
            config.instrument,
        );

        // Schedule the sequencer thread
        let timer = Timer::new();
        let guard = timer.schedule_repeating(
            Duration::milliseconds((60_000.0 / config.bpm as f32) as i64),
            move || thread.tick(),
        );
        guard.ignore();

        Sequencer {
            sender: tx,
            _timer: timer,
        }
    }

    pub fn start(&self) {
        self.sender.send(SequencerCommand::Start).unwrap();
    }

    pub fn stop(&self) {
        self.sender.send(SequencerCommand::Stop).unwrap();
    }

    fn build_pitch_producer(config: &SequencerConfiguration) -> Box<dyn PitchModule> {
        Box::new(PitchQuantizer::new(
            Box::new(RandomPitchProducer::new(
                LetterOctave(Letter::C, 3),
                LetterOctave(Letter::C, 5),
            )),
            config.quantizer_scale.clone(),
        ))
    }

    fn build_trigger_producer(config: &SequencerConfiguration) -> Box<dyn TriggerModule> {
        Box::new(RandomTriggerProducer::new())
    }

    pub fn update_instrument(&self, instrument: u8) {
        self.sender
            .send(SequencerCommand::SetInstrument(instrument))
            .unwrap();
    }

    pub fn update_pitch_producer(&self, config: SequencerConfiguration) {
        self.sender
            .send(SequencerCommand::SetPitchProducer(
                Sequencer::build_pitch_producer(&config),
            ))
            .unwrap();
    }

    pub fn update_trigger_producer(&self, config: SequencerConfiguration) {
        self.sender
            .send(SequencerCommand::SetTriggerProducer(
                Sequencer::build_trigger_producer(&config),
            ))
            .unwrap();
    }
}

struct SequencerThread {
    receiver: mpsc::Receiver<SequencerCommand>,
    pitch_Producer: Box<dyn PitchModule>,
    trigger_Producer: Box<dyn TriggerModule>,
    midi_output_conn: MidiOutputConnection,
    is_playing: bool,
    instrument: u8,
}

impl SequencerThread {
    fn new(
        receiver: mpsc::Receiver<SequencerCommand>,
        pitch_Producer: Box<dyn PitchModule>,
        trigger_Producer: Box<dyn TriggerModule>,
        is_playing: bool,
        instrument: u8,
    ) -> SequencerThread {
        // Create MIDI output
        let midi_out = midir::MidiOutput::new("Generative Sequencer").unwrap();

        // Connect to the first available MIDI output port (IAC Bus 1)
        let out_port = &midi_out.ports()[0];
        let out_conn = midi_out.connect(out_port, "Generative Sequencer").unwrap();

        SequencerThread {
            receiver,
            pitch_Producer,
            trigger_Producer,
            midi_output_conn: out_conn,
            is_playing: is_playing,
            instrument: instrument,
        }
    }

    fn tick(&mut self) {
        // Process all pending commands
        for command in self.receiver.try_iter() {
            match command {
                SequencerCommand::Start => {
                    if !self.is_playing {
                        self.is_playing = true
                    }
                }
                SequencerCommand::Stop => {
                    if self.is_playing {
                        self.is_playing = false
                    }
                }
                SequencerCommand::SetPitchProducer(pp) => {
                    self.pitch_Producer = pp;
                }
                SequencerCommand::SetTriggerProducer(tp) => {
                    self.trigger_Producer = tp;
                }
                SequencerCommand::SetInstrument(i) => {
                    self.instrument = i;
                }
            };
        }

        // Play note
        if self.is_playing {
            let pitch = self.pitch_Producer.tick();
            match self.trigger_Producer.tick() {
                Trigger::On => {
                    // Play the generated MIDI note
                    let note = pitch.step() as u8;

                    self.midi_output_conn
                        .send(&[PROGRAM_CHANGE_MSG, self.instrument])
                        .unwrap();

                    self.midi_output_conn
                        .send(&[NOTE_ON_MSG, note, VELOCITY])
                        .unwrap();
                    sleep(core::time::Duration::from_millis(100));
                    self.midi_output_conn
                        .send(&[NOTE_OFF_MSG, note, VELOCITY])
                        .unwrap();
                }
                Trigger::Off => (),
            }
        }
    }
}
