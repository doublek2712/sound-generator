use pitch_calc::*;
use rand::prelude::*;

// producers
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
