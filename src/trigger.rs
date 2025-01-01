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
