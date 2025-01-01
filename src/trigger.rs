use rand::prelude::*;

// constants
const RANDOM_PROBABILITY: f64 = 1.0;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Trigger {
    Off,
    On,
}

impl Trigger {
    pub fn from_bool(_bool: bool) -> Trigger {
        if _bool {
            Trigger::On
        } else {
            Trigger::Off
        }
    }
}

pub trait TriggerModule: Send + Sync {
    fn tick(&mut self) -> Trigger;
}

pub struct RandomTriggerProducer<R: Rng> {
    rng: R,
}

impl RandomTriggerProducer<SmallRng> {
    pub fn new() -> RandomTriggerProducer<SmallRng> {
        RandomTriggerProducer {
            rng: SmallRng::from_entropy(),
        }
    }
}

impl<R: Rng + Send + Sync> TriggerModule for RandomTriggerProducer<R> {
    fn tick(&mut self) -> Trigger {
        Trigger::from_bool(self.rng.gen_bool(RANDOM_PROBABILITY))
    }
}

pub struct ClockDivider {
    factor: u32,
    counter: u32,
    input: Box<dyn TriggerModule>,
}

impl ClockDivider {
    pub fn new(input: Box<dyn TriggerModule>, factor: u32) -> ClockDivider {
        ClockDivider {
            factor: factor,
            counter: 0,
            input: input,
        }
    }
}

impl TriggerModule for ClockDivider {
    fn tick(&mut self) -> Trigger {
        let trigger = if (self.counter == 0 || self.counter == self.factor) {
            self.counter = 0;
            self.input.tick()
        } else {
            Trigger::Off
        };
        self.counter += 1;
        trigger
    }
}

pub struct RhythmDivider {
    factor: u32,
    counter: u32,
    notes_per_beat: [u32; 4],
    current_beat_index: u32,
    current_beat_note: u32,

    input: Box<dyn TriggerModule>,
}
impl RhythmDivider {
    pub fn new(
        input: Box<dyn TriggerModule>,
        factor: u32,
        notes_per_beat: [u32; 4],
    ) -> RhythmDivider {
        RhythmDivider {
            factor: factor,
            counter: 0,
            notes_per_beat: notes_per_beat,
            current_beat_index: 0,
            current_beat_note: 0,
            input: input,
        }
    }
}

impl TriggerModule for RhythmDivider {
    fn tick(&mut self) -> Trigger {
        if self.current_beat_note == self.notes_per_beat[self.current_beat_index as usize]
            && self.counter == self.factor
        {
            if self.current_beat_index == 3 {
                self.current_beat_index = 0;
            } else {
                self.current_beat_index += 1;
            }
            self.current_beat_note = 0;
        }

        let trigger = if couter_calculation(
            self.counter.clone(),
            self.factor.clone(),
            self.notes_per_beat[self.current_beat_index as usize].clone(),
        ) {
            self.counter = 0;
            self.current_beat_note += 1;
            self.input.tick()
        } else {
            Trigger::Off
        };
        self.counter += 1;
        trigger
    }
}

fn couter_calculation(counter: u32, factor: u32, notes_per_beat: u32) -> bool {
    if counter == 0 && counter == factor {
        return true;
    }
    if counter % ((factor + notes_per_beat - 1) / notes_per_beat) == 0 {
        return true;
    }
    return false;
}
