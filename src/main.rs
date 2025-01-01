mod assets;
mod pitch;
mod sequencer;
mod trigger;

use assets::INSTRUMENT_LIST;
use nannou::prelude::*;
use nannou_egui::{
    egui::{self, RichText},
    Egui,
};
use pitch_calc::*;
use sequencer::*;

//constants
const WINDOW_NAME: &str = "Sound generator";

const INSTRUMENT_DEFAULT_VALUE: u8 = 10;
const BPM_DEFAULT_VALUE: f32 = 120.0;
const QUANTIZER_SCALE_INDEX_DEFAULT_VALUE: usize = 1;
const QUANTIZER_SCALES: &[(&[Letter], &str)] = &[
    (assets::CHROMATIC_SCALE_NOTES, "Chromatic"),
    (assets::MAJOR_SCALE_NOTES, "Major"),
    (assets::MINOR_SCALE_NOTES, "Minor"),
    (assets::MAJOR_PENTATONIC_SCALE_NOTES, "Major Pentatonic"),
    (assets::MINOR_PENTATONIC_SCALE_NOTES, "Minor Pentatonic"),
];

fn main() {
    nannou::app(model).update(update).run();
}
#[derive(Clone)]
struct SequencerModel {
    instrument: u8,
    quantizer_scale_index: Option<usize>,
    bpm: f32,
}
impl From<SequencerModel> for SequencerConfiguration {
    fn from(model: SequencerModel) -> Self {
        SequencerConfiguration {
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
    model
        .sequencer
        .update_pitch_producer(model.sequencer_model.clone().into());
}
fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(BLACK);
    draw.to_frame(app, &frame).unwrap();
    model.egui.draw_to_frame(&frame).unwrap();
}
