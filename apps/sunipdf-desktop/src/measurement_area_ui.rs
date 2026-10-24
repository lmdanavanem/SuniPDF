//! Shared area and volume draft controls; calculations remain in the core model.

use sunipdf_core::measurement::{Measurement, MeasurementDepth, MeasurementUnit};
use sunipdf_i18n::{tr, tr_args, Language};

pub(crate) fn ring_selector(
    ui: &mut egui::Ui,
    lang: Language,
    id: &str,
    selected: &mut Option<usize>,
    count: usize,
) {
    let label = |index: Option<usize>| {
        index.map_or_else(
            || tr(lang, "measure.deductions.outer").to_owned(),
            |index| {
                tr_args(
                    lang,
                    "measure.deductions.ring",
                    &[("number", &(index + 1).to_string())],
                )
            },
        )
    };
    egui::ComboBox::from_id_salt(id)
        .width(ui.available_width())
        .truncate()
        .selected_text(label(*selected))
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, None, label(None));
            for index in 0..count {
                ui.selectable_value(selected, Some(index), label(Some(index)));
            }
        });
}

pub(crate) fn depth_controls(
    ui: &mut egui::Ui,
    lang: Language,
    depth: &mut MeasurementDepth,
) -> bool {
    let old = *depth;
    ui.label(tr(lang, "measure.volume.depth"));
    ui.columns(2, |columns| {
        columns[0].add(
            egui::DragValue::new(&mut depth.value)
                .range(1e-9..=1e9)
                .speed(0.01)
                .max_decimals(9),
        );
        egui::ComboBox::from_id_salt("measurement-depth-unit")
            .width(columns[1].available_width())
            .selected_text(depth.unit.id())
            .show_ui(&mut columns[1], |ui| {
                for unit in MeasurementUnit::ALL {
                    ui.selectable_value(&mut depth.unit, unit, unit.id());
                }
            });
    });
    ui.small(tr(lang, "measure.volume.depth_hint"));
    old != *depth
}

pub(crate) fn area_summary(ui: &mut egui::Ui, lang: Language, measurement: &Measurement) {
    match measurement.area_breakdown() {
        Ok(area) => {
            let precision = measurement.precision;
            for (key, value) in [
                ("measure.deductions.gross", area.gross),
                (
                    "measure.deductions.deducted",
                    area.deductions
                        .iter()
                        .fold(0.0, |total, value| total + value),
                ),
                ("measure.deductions.net", area.net),
            ] {
                ui.label(format!(
                    "{}: {value:.precision$} {}²",
                    tr(lang, key),
                    measurement.unit.id()
                ));
            }
            if let Some(depth) = measurement.depth {
                ui.small(format!(
                    "{}: {} {}",
                    tr(lang, "measure.volume.depth"),
                    depth.value,
                    depth.unit.id()
                ));
                if let Ok(value) = measurement.value() {
                    ui.strong(format!(
                        "{}: {value:.precision$} {}³",
                        tr(lang, "measure.volume"),
                        measurement.unit.id()
                    ));
                }
            }
        }
        Err(_) => {
            ui.small(tr(lang, "measure.deductions.invalid"));
        }
    }
}
