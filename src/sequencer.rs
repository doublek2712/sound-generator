use std::{sync::mpsc, thread::sleep};

use chrono::Duration;
use pitch_calc::*;
use timer::Timer;

use midir::MidiOutputConnection;

use crate::assets::{NoteDurationLetter, NOTE_DURATION};
use crate::pitch::*;
use crate::trigger::*;

//constants
const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;
const PROGRAM_CHANGE_MSG: u8 = 0xC0;
const VELOCITY: u8 = 0x64;
const BPM: f32 = 60.0;
const TICKS_PER_QUARTER_NOTE: u32 = 40;
const CLOCK_DIVIDER_MAX: u32 = 32;
const CLOCK_DIVIDER_MIN: u32 = 1;
const SCHEDULE_REPEATING_DURATION: i64 = (60_000.0 / BPM / TICKS_PER_QUARTER_NOTE as f32) as i64;

pub struct SequencerConfiguration {
    pub min_pitch: LetterOctave,
    pub max_pitch: LetterOctave,
    pub pitch_producer_type: PitchProducerType,
    pub cycle_length: u32,
    pub rhythm_pattern: Vec<NoteDurationLetter>,
    pub notes_per_beat: [u32; 4],
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
    SetRhythmPattern(Vec<NoteDurationLetter>),
    SetTempo(f32),
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
            config.bpm,
            config.rhythm_pattern,
        );

        // Schedule the sequencer thread
        let timer = Timer::new();
        let guard = timer.schedule_repeating(
            Duration::milliseconds(SCHEDULE_REPEATING_DURATION),
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
        let pitch_producer: Box<dyn PitchModule> = match config.pitch_producer_type {
            PitchProducerType::Random => {
                Box::new(RandomPitchProducer::new(config.min_pitch, config.max_pitch))
            }

            PitchProducerType::RampUp => Box::new(RampPitchProducer::new(
                config.cycle_length,
                config.min_pitch,
                config.max_pitch,
            )),

            PitchProducerType::Square => Box::new(SquarePitchProducer::new(
                config.cycle_length,
                config.min_pitch,
                config.max_pitch,
            )),

            PitchProducerType::Sine => Box::new(SinePitchProducer::new(
                config.cycle_length,
                config.min_pitch,
                config.max_pitch,
            )),
        };
        Box::new(PitchQuantizer::new(
            pitch_producer,
            config.quantizer_scale.clone(),
        ))
    }

    fn build_trigger_producer(config: &SequencerConfiguration) -> Box<dyn TriggerModule> {
        Box::new(RhythmDivider::new(
            Box::new(RandomTriggerProducer::new()),
            (TICKS_PER_QUARTER_NOTE * BPM as u32) / config.bpm as u32,
            config.notes_per_beat,
        ))
    }

    pub fn update_instrument(&self, instrument: u8) {
        self.sender
            .send(SequencerCommand::SetInstrument(instrument))
            .unwrap();
    }

    pub fn update_rhythm_pattern(&self, rhythm_pattern: Vec<NoteDurationLetter>) {
        self.sender
            .send(SequencerCommand::SetRhythmPattern(rhythm_pattern))
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
        self.sender
            .send(SequencerCommand::SetTempo(config.bpm))
            .unwrap();
    }
}

struct SequencerThread {
    receiver: mpsc::Receiver<SequencerCommand>,
    pitch_producer: Box<dyn PitchModule>,
    trigger_producer: Box<dyn TriggerModule>,
    midi_output_conn: MidiOutputConnection,
    is_playing: bool,
    instrument: u8,
    tempo: f32,
    rhythm_pattern: Vec<NoteDurationLetter>,
    current_rhythm_index: usize,
}

impl SequencerThread {
    fn new(
        receiver: mpsc::Receiver<SequencerCommand>,
        pitch_producer: Box<dyn PitchModule>,
        trigger_producer: Box<dyn TriggerModule>,
        is_playing: bool,
        instrument: u8,
        tempo: f32,
        rhythm_pattern: Vec<NoteDurationLetter>,
    ) -> SequencerThread {
        // Create MIDI output
        let midi_out = midir::MidiOutput::new("Generative Sequencer").unwrap();

        // Connect to the first available MIDI output port (IAC Bus 1)
        let out_port = &midi_out.ports()[0];
        let out_conn = midi_out.connect(out_port, "Generative Sequencer").unwrap();

        SequencerThread {
            receiver,
            pitch_producer,
            trigger_producer,
            midi_output_conn: out_conn,
            is_playing,
            instrument,
            tempo,
            rhythm_pattern,
            current_rhythm_index: 0,
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
                    self.pitch_producer = pp;
                }
                SequencerCommand::SetTriggerProducer(tp) => {
                    self.trigger_producer = tp;
                }
                SequencerCommand::SetInstrument(i) => {
                    self.instrument = i;
                }
                SequencerCommand::SetRhythmPattern(rp) => {
                    self.rhythm_pattern = rp;
                    self.current_rhythm_index = 0;
                }
                SequencerCommand::SetTempo(t) => {
                    self.tempo = t;
                }
            };
        }

        // Play note
        if self.is_playing {
            let pitch = self.pitch_producer.tick();
            match self.trigger_producer.tick() {
                Trigger::On => {
                    // Play the generated MIDI note
                    let note = pitch.step() as u8;

                    self.midi_output_conn
                        .send(&[PROGRAM_CHANGE_MSG, self.instrument])
                        .unwrap();

                    self.midi_output_conn
                        .send(&[NOTE_ON_MSG, note, VELOCITY])
                        .unwrap();
                    let note_duration_letter = &self.rhythm_pattern[self.current_rhythm_index];
                    let note_duration = NOTE_DURATION[note_duration_letter.clone() as usize];
                    sleep(core::time::Duration::from_millis(
                        (note_duration * 60_000.0 / self.tempo as f32) as u64,
                    ));
                    self.midi_output_conn
                        .send(&[NOTE_OFF_MSG, note, VELOCITY])
                        .unwrap();
                    self.current_rhythm_index =
                        (self.current_rhythm_index + 1) % self.rhythm_pattern.len();
                }
                Trigger::Off => (),
            }
        }
    }
}
