use pitch_calc::*;
use rand::prelude::*;
use std::{f32::consts::PI, fmt::Display, str::FromStr};

// producers
#[derive(PartialEq)]
pub enum PitchProducerType {
    Random,
    RampUp,
    Square,
    Sine,
}

impl Display for PitchProducerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PitchProducerType::Random => write!(f, "Random"),
            PitchProducerType::RampUp => write!(f, "Ramp"),
            PitchProducerType::Square => write!(f, "Square"),
            PitchProducerType::Sine => write!(f, "Sine"),
        }
    }
}

impl FromStr for PitchProducerType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Random" => Ok(PitchProducerType::Random),
            "Ramp" => Ok(PitchProducerType::RampUp),
            "Square" => Ok(PitchProducerType::Square),
            "Sine" => Ok(PitchProducerType::Sine),
            _ => Err(()),
        }
    }
}
pub trait PitchModule: Send + Sync {
    fn tick(&mut self) -> LetterOctave;
}

pub struct RandomPitchProducer<R: Rng + Send + Sync> {
    rng: R,
    min: f32,
    max: f32,
}

impl RandomPitchProducer<SmallRng> {
    pub fn new(min: LetterOctave, max: LetterOctave) -> RandomPitchProducer<SmallRng> {
        RandomPitchProducer {
            rng: SmallRng::from_entropy(),
            min: min.step(),
            max: max.step(),
        }
    }
}

impl<R: Rng + Send + Sync> PitchModule for RandomPitchProducer<R> {
    fn tick(&mut self) -> LetterOctave {
        if self.min != self.max {
            let r: f32 = self.rng.gen_range(self.min..self.max);
            Step(r).to_letter_octave()
        } else {
            Step(self.min).to_letter_octave()
        }
    }
}

pub struct RampPitchProducer {
    cycle_length: u32,
    min: f32,
    max: f32,
    counter: u32,
}

impl RampPitchProducer {
    pub fn new(cycle_length: u32, min: LetterOctave, max: LetterOctave) -> RampPitchProducer {
        RampPitchProducer {
            cycle_length: cycle_length,
            min: min.step(),
            max: max.step(),
            counter: 0,
        }
    }
}

impl PitchModule for RampPitchProducer {
    fn tick(&mut self) -> LetterOctave {
        let slope = if self.cycle_length > 1 {
            (self.max - self.min) / (self.cycle_length - 1) as f32
        } else {
            0.
        };
        let step = Step(self.min + slope * self.counter as f32);
        let pitch = step.to_letter_octave();
        if self.counter == self.cycle_length - 1 {
            self.counter = 0;
        } else {
            self.counter += 1;
        }
        pitch
    }
}

pub struct SquarePitchProducer {
    cycle_length: u32,
    min: f32,
    max: f32,
    counter: u32,
}

impl SquarePitchProducer {
    pub fn new(cycle_length: u32, min: LetterOctave, max: LetterOctave) -> SquarePitchProducer {
        SquarePitchProducer {
            cycle_length: cycle_length,
            min: min.step(),
            max: max.step(),
            counter: 0,
        }
    }
}

impl PitchModule for SquarePitchProducer {
    fn tick(&mut self) -> LetterOctave {
        self.counter += 1;
        let pitch = if self.counter <= self.cycle_length / 2 {
            Step(self.min).to_letter_octave()
        } else {
            if self.counter == self.cycle_length {
                self.counter = 0;
            }
            Step(self.max).to_letter_octave()
        };
        pitch
    }
}

pub struct SinePitchProducer {
    cycle_length: u32,
    min: f32,
    max: f32,
    counter: u32,
}

impl SinePitchProducer {
    pub fn new(cycle_length: u32, min: LetterOctave, max: LetterOctave) -> SinePitchProducer {
        SinePitchProducer {
            cycle_length: cycle_length,
            min: min.step(),
            max: max.step(),
            counter: 0,
        }
    }
}

impl PitchModule for SinePitchProducer {
    fn tick(&mut self) -> LetterOctave {
        // Calculate the angle in radians
        let angle = 2.0 * PI * self.counter as f32 / self.cycle_length as f32;

        // Calculate the sine value, map it to [min, max]
        let normalized_sine: f32 = (angle.sin() + 1.0) / 2.0; // Map sin [-1, 1] to [0, 1]
        let pitch = self.min + (self.max - self.min) * normalized_sine;

        // Convert to LetterOctave
        let step = Step(pitch).to_letter_octave();

        // Update counter
        self.counter = (self.counter + 1) % self.cycle_length;

        step
    }
}

//quantizer
pub struct PitchQuantizer {
    input: Box<dyn PitchModule>,
    scale: Vec<Letter>,
}

impl PitchQuantizer {
    pub fn new(input: Box<dyn PitchModule>, scale: Vec<Letter>) -> PitchQuantizer {
        PitchQuantizer { input, scale }
    }
}

impl PitchModule for PitchQuantizer {
    fn tick(&mut self) -> LetterOctave {
        let unquantized_note = self.input.tick();
        self.scale.sort();
        for letter in &self.scale {
            if *letter == unquantized_note.letter() {
                return unquantized_note;
            } else if *letter > unquantized_note.letter() {
                // quantize up to the next note in scale
                let quantized_note = LetterOctave(letter.clone(), unquantized_note.octave());
                return quantized_note;
            }
        }

        // handle case when the unquantized note is above the highest note in scale by wrapping around
        let quantized = LetterOctave(self.scale[0], unquantized_note.octave() + 1);
        return quantized;
    }
}
