mod assets;
mod pitch;
mod sequencer;
mod trigger;

use std::str::FromStr;

use assets::{format_letter_octave, NoteDurationLetter, INSTRUMENT_LIST};
use nannou::prelude::*;
use nannou_egui::{
    egui::{self, RichText},
    Egui,
};
use pitch::PitchProducerType;
use pitch_calc::*;
use sequencer::*;

//constants
const WINDOW_NAME: &str = "Sound generator";

const INSTRUMENT_DEFAULT_VALUE: u8 = 10;
const BPM_DEFAULT_VALUE: f32 = 160.0;
const MIN_BPM_VALUE: f32 = 60.0;
const MAX_BPM_VALUE: f32 = 240.0;
const QUANTIZER_SCALE_INDEX_DEFAULT_VALUE: usize = 1;
const QUANTIZER_SCALES: &[(&[Letter], &str)] = &[
    (assets::CHROMATIC_SCALE_NOTES, "Chromatic"),
    (assets::MAJOR_SCALE_NOTES, "Major"),
    (assets::MINOR_SCALE_NOTES, "Minor"),
    (assets::MAJOR_PENTATONIC_SCALE_NOTES, "Major Pentatonic"),
    (assets::MINOR_PENTATONIC_SCALE_NOTES, "Minor Pentatonic"),
];

const DEFAULT_CYCLE_LENGTH: u32 = 64;
const MIN_CYCLE_LENGTH: u32 = 16;
const MAX_CYCLE_LENGTH: u32 = 128;
const PITCH_MIN_VALUE: LetterOctave = LetterOctave(Letter::C, 0);
const PITCH_MAX_VALUE: LetterOctave = LetterOctave(Letter::C, 7);
const MIN_PITCH_DEFAULT_VALUE: LetterOctave = LetterOctave(Letter::C, 3);
const MAX_PITCH_DEFAULT_VALUE: LetterOctave = LetterOctave(Letter::C, 5);
const PITCH_PRODUCER_TYPE_DEFAULT_VALUE: usize = 0;
const PITCH_PRODUCER_TYPE_NAMES: &[&str] = &["Ramp", "Square", "Sine", "Random"];

const RHYTHM_PATTERNS: &[(&[NoteDurationLetter], &str)] = &[
    (assets::STRAIGHT_RHYTHM_PATTERN, "Straight"),
    (assets::SYNCOPATED_RHYTHM_PATTERN, "Syncopated"),
    (assets::FAST_RHYTHM_PATTERN, "Fast"),
    (assets::LONG_AND_SHORT_RHYTHM_PATTERN, "Long and Short"),
    (assets::COMPLEX_RHYTHM_PATTERN, "Complex"),
];
const RHYTHM_PATTERN_DEFAULT_VALUE: usize = 0;
const NOTES_PER_BEAT: &[[u32; 4]] = &[
    assets::BEAT_PER_BAR_DIVIDE_FOR_FOUR,
    assets::BEAT_PER_BAR_DIVIDE_FOR_SIX,
    assets::BEAT_PER_BAR_DIVIDE_FOR_EIGTH,
    assets::BEAT_PER_BAR_DIVIDE_FOR_FOUR,
    assets::BEAT_PER_BAR_DIVIDE_FOR_SEVEN,
];

fn main() {
    nannou::app(model).update(update).run();
}
#[derive(Clone)]
struct SequencerModel {
    min_pitch: f32,
    max_pitch: f32,
    pitch_producer_type_index: Option<usize>,
    cycle_length: f32,
    rhythm_pattern: Option<usize>,
    notes_per_beat: [u32; 4],
    instrument: u8,
    quantizer_scale_index: Option<usize>,
    bpm: f32,
}
impl From<SequencerModel> for SequencerConfiguration {
    fn from(model: SequencerModel) -> Self {
        SequencerConfiguration {
            min_pitch: Step(model.min_pitch).to_letter_octave(),
            max_pitch: Step(model.max_pitch).to_letter_octave(),
            pitch_producer_type: pitch_producer_type_from_index(model.pitch_producer_type_index),
            cycle_length: model.cycle_length as u32,
            rhythm_pattern: RHYTHM_PATTERNS[model.rhythm_pattern.unwrap()].0.to_vec(),
            notes_per_beat: NOTES_PER_BEAT[model.rhythm_pattern.unwrap()],
            instrument: model.instrument,
            quantizer_scale: QUANTIZER_SCALES[model.quantizer_scale_index.unwrap()]
                .0
                .to_vec(),
            bpm: model.bpm,
        }
    }
}

struct Model {
    egui: Egui,
    sequencer_model: SequencerModel,
    sequencer: Sequencer,
    is_playing: bool,
}

fn model(app: &App) -> Model {
    // Create window
    let window_id = app
        .new_window()
        .title(WINDOW_NAME)
        .size(300, 300)
        .view(view)
        .raw_event(raw_window_event)
        .build()
        .unwrap();
    let window = app.window(window_id).unwrap();

    let egui = Egui::from_window(&window);

    let sequencer_model = SequencerModel {
        min_pitch: MIN_PITCH_DEFAULT_VALUE.step(),
        max_pitch: MAX_PITCH_DEFAULT_VALUE.step(),
        pitch_producer_type_index: Some(PITCH_PRODUCER_TYPE_DEFAULT_VALUE),
        cycle_length: DEFAULT_CYCLE_LENGTH as f32,
        rhythm_pattern: Some(RHYTHM_PATTERN_DEFAULT_VALUE),
        notes_per_beat: NOTES_PER_BEAT[RHYTHM_PATTERN_DEFAULT_VALUE],
        instrument: INSTRUMENT_DEFAULT_VALUE,
        quantizer_scale_index: Some(QUANTIZER_SCALE_INDEX_DEFAULT_VALUE),
        bpm: BPM_DEFAULT_VALUE,
    };

    let is_playing = true;
    let sequencer = Sequencer::new(sequencer_model.clone().into(), is_playing);

    Model {
        egui,
        sequencer_model,
        sequencer,
        is_playing,
    }
}
fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);
}

fn update(app: &App, model: &mut Model, update: Update) {
    let egui = &mut model.egui;

    egui.set_elapsed_time(update.since_start);
    let ctx = egui.begin_frame();
    let scale = &mut model.sequencer_model.quantizer_scale_index;
    let mut pitch_producer_type = model.sequencer_model.pitch_producer_type_index.clone();
    let mut tempo = model.sequencer_model.bpm.clone();
    let mut min_pitch = model.sequencer_model.min_pitch.clone();
    let mut max_pitch = model.sequencer_model.max_pitch.clone();
    let mut cycle_length = model.sequencer_model.cycle_length.clone();
    let mut rhythm_pattern = model.sequencer_model.rhythm_pattern.clone();
    let instrument = &mut model.sequencer_model.instrument;

    egui::Window::new("Settings")
        .default_width(250.0)
        .show(&ctx, |ui| {
            egui::Grid::new("my_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Scale:");
                    egui::ComboBox::from_id_source("scale")
                        .selected_text(format!("{}", QUANTIZER_SCALES[scale.unwrap()].1))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for (index, (_, name)) in QUANTIZER_SCALES.iter().enumerate() {
                                ui.selectable_value(scale, Some(index), *name);
                            }
                        });
                    ui.end_row();
                    ui.label("Rhythm:");
                    egui::ComboBox::from_id_source("rhythm")
                        .selected_text(format!("{}", RHYTHM_PATTERNS[rhythm_pattern.unwrap()].1))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for (index, (_, name)) in RHYTHM_PATTERNS.iter().enumerate() {
                                ui.selectable_value(&mut rhythm_pattern, Some(index), *name);
                            }
                        });
                    ui.end_row();
                    ui.label("Pitch:");
                    egui::ComboBox::from_id_source("pitch")
                        .selected_text(format!(
                            "{}",
                            PITCH_PRODUCER_TYPE_NAMES[pitch_producer_type.unwrap()]
                        ))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for (index, name) in PITCH_PRODUCER_TYPE_NAMES.iter().enumerate() {
                                ui.selectable_value(&mut pitch_producer_type, Some(index), *name);
                            }
                        });
                    ui.end_row();
                    ui.label("Cycle length:");
                    ui.add(egui::Slider::new(
                        &mut cycle_length,
                        MIN_CYCLE_LENGTH as f32..=MAX_CYCLE_LENGTH as f32,
                    ));
                    ui.end_row();
                    ui.label("Min:");
                    ui.add(
                        egui::Slider::new(&mut min_pitch, PITCH_MIN_VALUE.step()..=max_pitch).text(
                            format_letter_octave(
                                Step(model.sequencer_model.min_pitch).to_letter_octave(),
                            ),
                        ),
                    );
                    ui.end_row();
                    ui.label("Max:");
                    ui.add(
                        egui::Slider::new(&mut max_pitch, min_pitch..=PITCH_MAX_VALUE.step()).text(
                            format_letter_octave(
                                Step(model.sequencer_model.max_pitch).to_letter_octave(),
                            ),
                        ),
                    );
                    ui.end_row();

                    ui.label("Tempo:");
                    ui.add(egui::Slider::new(&mut tempo, MIN_BPM_VALUE..=MAX_BPM_VALUE));
                    ui.end_row();
                    ui.label("Instrument:");
                    egui::ComboBox::from_id_source("instrument")
                        .selected_text(format!("{}", INSTRUMENT_LIST[*instrument as usize]))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for (index, (name)) in INSTRUMENT_LIST.iter().enumerate() {
                                ui.selectable_value(instrument, index as u8, *name);
                            }
                        });
                    ui.end_row();
                });
            ui.separator();

            let play_text = if model.is_playing { "Pause" } else { "Play" };

            if ui
                .add(egui::Button::new(RichText::new(play_text).heading()))
                .clicked()
            {
                if model.is_playing {
                    model.sequencer.stop();
                    model.is_playing = false;
                } else {
                    model.sequencer.start();
                    model.is_playing = true;
                }
            };
        });

    // Update changes
    model
        .sequencer
        .update_instrument(model.sequencer_model.instrument);
    if model.sequencer_model.rhythm_pattern != rhythm_pattern {
        model.sequencer_model.rhythm_pattern = rhythm_pattern;
        model.sequencer_model.notes_per_beat = NOTES_PER_BEAT[rhythm_pattern.unwrap()];

        model.sequencer.update_rhythm_pattern(
            RHYTHM_PATTERNS[model.sequencer_model.rhythm_pattern.unwrap()]
                .0
                .to_vec(),
        );
        model
            .sequencer
            .update_trigger_producer(model.sequencer_model.clone().into());
    }

    if (model.sequencer_model.pitch_producer_type_index != pitch_producer_type) {
        model.sequencer_model.pitch_producer_type_index = pitch_producer_type;
        model
            .sequencer
            .update_pitch_producer(model.sequencer_model.clone().into());
    }
    if (model.sequencer_model.min_pitch != min_pitch) {
        model.sequencer_model.min_pitch = min_pitch;
        model
            .sequencer
            .update_pitch_producer(model.sequencer_model.clone().into());
    }
    if (model.sequencer_model.max_pitch != max_pitch) {
        model.sequencer_model.max_pitch = max_pitch;
        model
            .sequencer
            .update_pitch_producer(model.sequencer_model.clone().into());
    }
    if (model.sequencer_model.cycle_length != cycle_length) {
        model.sequencer_model.cycle_length = cycle_length;
        model
            .sequencer
            .update_pitch_producer(model.sequencer_model.clone().into());
    }
    if (model.sequencer_model.bpm != tempo) {
        model.sequencer_model.bpm = tempo;
        model
            .sequencer
            .update_trigger_producer(model.sequencer_model.clone().into());
    }
}
fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(BLACK);
    draw.to_frame(app, &frame).unwrap();
    model.egui.draw_to_frame(&frame).unwrap();
}

fn pitch_producer_type_from_index(idx: Option<usize>) -> PitchProducerType {
    PitchProducerType::from_str(PITCH_PRODUCER_TYPE_NAMES[idx.unwrap()]).unwrap()
}
