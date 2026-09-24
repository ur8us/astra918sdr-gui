use crate::worker::{Request, Snapshot, Worker};
use astra918_firmware::{
    astra as a,
    cat::Mode,
    control_v2 as v2,
    controls::{
        IF_GAIN_DB10, LF_ATTENUATOR_DB10, LF_LNA_GAIN_DB10, LF_MIX_GAIN_DB10, RF_GAIN_DB10, RfInput,
    },
};
use egui::{Color32, RichText};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

fn single_receiver(devices: &[astra918_host::Device]) -> Option<&str> {
    match devices {
        [device] if !device.serial.is_empty() => Some(&device.serial),
        _ => None,
    }
}

#[derive(Default)]
pub struct Editor {
    pub text: String,
    pub dirty: bool,
    last: Option<i64>,
}
impl Editor {
    pub fn readback(&mut self, value: i64) {
        if self.last != Some(value) {
            if !self.dirty {
                self.text = value.to_string();
            }
            self.last = Some(value);
        }
    }
    fn commit(&mut self) -> Option<i64> {
        let value = self.text.trim().parse().ok()?;
        self.dirty = false;
        Some(value)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    simulator: bool,
    target: String,
    serial: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            simulator: false,
            target: "127.0.0.1:7350".into(),
            serial: String::new(),
        }
    }
}
pub struct App {
    worker: Worker,
    snapshot: Snapshot,
    preferences: Preferences,
    dial: Editor,
    offset: Editor,
    low: Editor,
    high: Editor,
    capacitor: u16,
    capacitor_readback: Option<u16>,
    capacitor_active: bool,
    capacitor_pending: bool,
    last_capacitor: Option<Instant>,
    auto_connect_pending: bool,
    content_height: Option<f32>,
    smoke_frames: Option<u32>,
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, simulator: Option<String>, smoke: bool) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let mut preferences: Preferences = cc
            .storage
            .and_then(|s| eframe::get_value(s, "astra.preferences"))
            .unwrap_or_default();
        if !cfg!(debug_assertions) {
            preferences.simulator = false;
        }
        let mut app = Self {
            worker: Worker::spawn(),
            snapshot: Snapshot::default(),
            preferences,
            dial: Editor::default(),
            offset: Editor::default(),
            low: Editor::default(),
            high: Editor::default(),
            capacitor: 0,
            capacitor_readback: None,
            capacitor_active: false,
            capacitor_pending: false,
            last_capacitor: None,
            auto_connect_pending: true,
            content_height: None,
            smoke_frames: smoke.then_some(80),
        };
        if let Some(address) = simulator {
            app.preferences.simulator = true;
            app.preferences.target = address;
            app.connect();
        } else {
            app.auto_connect_pending = !app.preferences.simulator;
            app.request(Request::Discover);
        }
        app
    }
    fn request(&mut self, request: Request) -> bool {
        if self.worker.requests.try_send(request).is_err() {
            self.snapshot.message = "A command is still pending; try again shortly".into();
            false
        } else {
            true
        }
    }
    fn command(&mut self, cmd: u8, payload: Vec<u8>) {
        self.request(Request::Command(cmd, payload));
    }
    fn connect(&mut self) {
        self.auto_connect_pending = false;
        self.capacitor_readback = None;
        self.capacitor_active = false;
        self.capacitor_pending = false;
        self.request(Request::Connect {
            simulator: self.preferences.simulator,
            target: if self.preferences.simulator {
                self.preferences.target.clone()
            } else {
                self.preferences.serial.clone()
            },
        });
    }
    fn editor(ui: &mut egui::Ui, label: &str, e: &mut Editor, button: &str) -> Option<i64> {
        let mut apply = false;
        ui.horizontal(|ui| {
            ui.label(label);
            let response = ui.add(egui::TextEdit::singleline(&mut e.text).desired_width(155.));
            if response.changed() {
                e.dirty = true;
            }
            apply = ui.button(button).clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        });
        if apply { e.commit() } else { None }
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        let Some(r) = self.snapshot.radio else {
            ui.label("Connect a receiver to access its controls.");
            return;
        };
        let s = r.settings;
        self.dial.readback(s.dial as i64);
        self.offset.readback(i64::from(s.offset));
        self.low.readback(i64::from(s.low));
        self.high.readback(i64::from(s.high));
        ui.heading("Tuning");
        if let Some(hz) = Self::editor(ui, "Receive frequency (Hz)", &mut self.dial, "Tune") {
            if hz >= 0 {
                self.command(v2::FREQUENCY_SET, (hz as u64).to_le_bytes().to_vec());
            } else {
                self.snapshot.message = "Frequency must be positive".into();
            }
        }
        if let Some(hz) = Self::editor(ui, "Channel offset (Hz)", &mut self.offset, "Apply") {
            if let Ok(hz) = i32::try_from(hz) {
                self.command(a::OFFSET, hz.to_le_bytes().to_vec());
            } else {
                self.snapshot.message = "Offset is out of range".into();
            }
        }
        ui.label(format!(
            "Applied dial: {:.6} MHz   Spectrum center: {:.6} MHz",
            s.dial as f64 / 1e6,
            s.center() as f64 / 1e6
        ));
        ui.small("Tuning keeps the offset. Changing the offset keeps the receive frequency.");
        ui.separator();
        ui.heading("USB audio for WSJT-X");
        ui.horizontal(|ui| {
            ui.label("Mode");
            for (label, mode) in [("USB", Mode::Usb), ("LSB", Mode::Lsb)] {
                if ui.selectable_label(s.mode == mode, label).clicked() {
                    self.command(a::MODE, vec![mode.digit() - b'0']);
                }
            }
            ui.label("12 kHz mono");
        });
        ui.horizontal(|ui| {
            ui.label("Audio passband (Hz)");
            if ui
                .add(egui::TextEdit::singleline(&mut self.low.text).desired_width(65.))
                .changed()
            {
                self.low.dirty = true;
            }
            ui.label("to");
            if ui
                .add(egui::TextEdit::singleline(&mut self.high.text).desired_width(65.))
                .changed()
            {
                self.high.dirty = true;
            }
            if ui.button("Apply filter").clicked()
                && let (Some(low), Some(high)) = (self.low.commit(), self.high.commit())
                && let (Ok(low), Ok(high)) = (u16::try_from(low), u16::try_from(high))
            {
                let mut p = low.to_le_bytes().to_vec();
                p.extend_from_slice(&high.to_le_bytes());
                self.command(a::AUDIO_FILTER, p);
            }
        });
        ui.small(format!(
            "Applied filter: {}–{} Hz; independent of SDR++ listening mode",
            s.low, s.high
        ));
        ui.separator();
        ui.heading("Receiver");
        ui.horizontal(|ui| {
            ui.label("Antenna input");
            for (index, label) in ["Auto", "LF", "HF", "VHF"].iter().enumerate() {
                if ui
                    .selectable_label(s.controls.input as usize == index, *label)
                    .clicked()
                {
                    self.command(v2::INPUT_SET, vec![index as u8]);
                }
            }
        });
        let resolved = s.controls.input.actual(s.hardware().frequency);
        ui.small(format!("Applied route: {resolved:?}"));
        ui.horizontal(|ui| {
            let mut auto = s.controls.rf_auto;
            if ui.checkbox(&mut auto, "Automatic RF gain").changed() {
                self.command(v2::GAIN_MODE, vec![0, (!auto) as u8]);
            }
            let mut auto = s.controls.if_auto;
            if ui.checkbox(&mut auto, "Automatic IF gain").changed() {
                self.command(v2::GAIN_MODE, vec![1, (!auto) as u8]);
            }
        });
        if !s.controls.rf_auto {
            if resolved == RfInput::Lf {
                self.gain(
                    ui,
                    "LF LNA + mixer",
                    2,
                    s.controls.lf_gain,
                    &(0..16)
                        .map(|i| LF_LNA_GAIN_DB10[i] + LF_MIX_GAIN_DB10[i])
                        .collect::<Vec<_>>(),
                );
                self.gain(
                    ui,
                    "LF attenuator",
                    3,
                    s.controls.lf_attenuator,
                    &LF_ATTENUATOR_DB10,
                );
            } else {
                self.gain(ui, "RF gain", 0, s.controls.rf_gain, &RF_GAIN_DB10);
            }
        }
        if !s.controls.if_auto {
            self.gain(ui, "IF gain", 1, s.controls.if_gain, &IF_GAIN_DB10);
        }
        let applied = s.controls.lf_mf_capacitor;
        if self.capacitor_readback != Some(applied)
            && !self.capacitor_active
            && !self.capacitor_pending
        {
            self.capacitor = applied;
        }
        self.capacitor_readback = Some(applied);
        let response = ui.add(
            egui::Slider::new(&mut self.capacitor, 0..=4095)
                .clamping(egui::SliderClamping::Always)
                .text("LF/MF capacitor (0-4095)"),
        );
        self.capacitor_pending |= response.changed();
        self.capacitor_active = response.dragged();
        if self.capacitor_pending
            && (!self.capacitor_active
                || self
                    .last_capacitor
                    .is_none_or(|t| t.elapsed() >= Duration::from_millis(100)))
            && self.request(Request::Command(
                v2::LF_MF_CAPACITOR_SET,
                self.capacitor.to_le_bytes().to_vec(),
            ))
        {
            self.capacitor_pending = false;
            self.last_capacitor = Some(Instant::now());
        }
        ui.small(format!(
            "Applied capacitor: {:.1} pF",
            3. + 241. * f64::from(s.controls.lf_mf_capacitor) / 4095.
        ));
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Save to receiver").clicked() {
                self.command(a::SAVE, vec![]);
            }
            if ui.button("Retry receiver").clicked() {
                self.command(a::RETRY, vec![]);
            }
            ui.label(if r.saved_revision == r.revision {
                "Saved"
            } else {
                "Unsaved changes"
            });
        });
        ui.small("Save persists all current settings and briefly interrupts reception.");
        ui.separator();
        ui.heading("Status");
        ui.label(format!(
            "{}   I/Q: {}   Revision: {}",
            if r.configured {
                "Receiver ready"
            } else {
                "Receiver fault"
            },
            if r.streaming { "streaming" } else { "stopped" },
            r.revision
        ));
        ui.label(format!(
            "I/Q drops: {}   Capture faults: {}   USB faults: {}",
            r.dropped, r.capture_faults, r.usb_faults
        ));
        ui.label(format!(
            "Audio underruns: {}   Overruns: {}   USB stalls: {}",
            r.underruns, r.overruns, r.audio_stalls
        ));
        ui.label(format!(
            "120 ksps I/Q; filter roll-off near spectrum edges. Error: {}",
            r.error
        ));
    }
    fn gain(&mut self, ui: &mut egui::Ui, label: &str, block: u8, code: u8, table: &[i16]) {
        ui.horizontal(|ui| {
            ui.label(label);
            egui::ComboBox::from_id_salt(("gain", block))
                .selected_text(format!(
                    "{:.1} dB (code {code})",
                    f32::from(table[code as usize]) / 10.
                ))
                .show_ui(ui, |ui| {
                    for (value, db) in table.iter().enumerate() {
                        if ui
                            .selectable_label(
                                value == code as usize,
                                format!("{:.1} dB (code {value})", f32::from(*db) / 10.),
                            )
                            .clicked()
                        {
                            self.command(v2::GAIN_SET, vec![block, value as u8]);
                        }
                    }
                });
        });
    }
}
impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "astra.preferences", &self.preferences);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(snapshot) = self.worker.snapshots.try_recv() {
            self.snapshot = snapshot;
        }
        if self.auto_connect_pending && self.snapshot.discovered {
            self.auto_connect_pending = false;
            if let Some(serial) = single_receiver(&self.snapshot.devices) {
                self.preferences.serial = serial.to_owned();
                self.connect();
            }
        }
        let panel = egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Astra918 receiver");
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.snapshot.connected, |ui| {
                        #[cfg(debug_assertions)]
                        ui.checkbox(&mut self.preferences.simulator, "Offline simulator");
                        if self.preferences.simulator {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.preferences.target)
                                    .desired_width(180.),
                            );
                        } else {
                            egui::ComboBox::from_id_salt("receiver")
                                .selected_text(if self.preferences.serial.is_empty() {
                                    "Select receiver"
                                } else {
                                    &self.preferences.serial
                                })
                                .show_ui(ui, |ui| {
                                    for d in &self.snapshot.devices {
                                        ui.selectable_value(
                                            &mut self.preferences.serial,
                                            d.serial.clone(),
                                            &d.label,
                                        );
                                    }
                                });
                            if ui.button("Refresh").clicked() {
                                self.request(Request::Discover);
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    if self.snapshot.connected {
                        if ui.button("Disconnect").clicked() {
                            self.request(Request::Disconnect);
                        }
                    } else if ui.button("Connect").clicked() {
                        self.connect();
                    }
                    ui.label(RichText::new(&self.snapshot.message).color(
                        if self.snapshot.connected {
                            Color32::from_rgb(30, 90, 55)
                        } else {
                            Color32::from_rgb(100, 65, 30)
                        },
                    ));
                });
                ui.separator();
                self.controls(ui);
            })
        });
        let content_height = panel.inner.content_size.y;
        if self
            .content_height
            .is_none_or(|last| (last - content_height).abs() > 1.)
        {
            let height = (content_height + 24.).clamp(180., 950.);
            let (available, width) = ctx.input(|i| {
                let viewport = i.viewport();
                (
                    viewport.monitor_size.unwrap_or(egui::vec2(1280., 900.)),
                    viewport.inner_rect.map_or(660., |r| r.width()),
                )
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                width,
                height.min(available.y - 80.),
            )));
            self.content_height = Some(content_height);
        }
        if let Some(left) = &mut self.smoke_frames {
            *left = left.saturating_sub(1);
            if *left == 0 {
                eprintln!(
                    "GUI smoke: connected={} revision={:?}",
                    self.snapshot.connected,
                    self.snapshot.radio.map(|s| s.revision)
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_connection_requires_one_identifiable_receiver() {
        use astra918_host::Device;
        let device = |serial: &str| Device {
            serial: serial.into(),
            label: serial.into(),
        };
        assert_eq!(single_receiver(&[]), None);
        assert_eq!(single_receiver(&[device("")]), None);
        assert_eq!(single_receiver(&[device("first")]), Some("first"));
        assert_eq!(single_receiver(&[device("first"), device("second")]), None);
    }
    #[test]
    fn polling_preserves_dirty_frequency_draft() {
        let mut e = Editor::default();
        e.readback(100);
        e.text = "123".into();
        e.dirty = true;
        e.readback(200);
        assert_eq!(e.text, "123");
        assert_eq!(e.commit(), Some(123));
        e.readback(123);
        assert_eq!(e.text, "123");
        e.readback(300);
        assert_eq!(e.text, "300");
    }
}
