use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use std::collections::HashMap;

use crate::CANFrameRecordDTO;

pub struct PlotApp {
    records: Vec<CANFrameRecordDTO>,
    plot_data: HashMap<u32, Vec<[f64; 2]>>,
    autoscroll: bool,
    current_idx: usize,
    playback_speed: f64,
    is_playing: bool,
    base_time_us: i64,
}

impl PlotApp {
    pub fn new(records: Vec<CANFrameRecordDTO>) -> Self {
        let base_time_us = records.first().map(|r| r.hw_ts_us).unwrap_or(0);
        Self {
            records,
            plot_data: HashMap::new(),
            autoscroll: true,
            current_idx: 0,
            playback_speed: 1.0,
            is_playing: false,
            base_time_us,
        }
    }

    fn add_frame(&mut self, record: &CANFrameRecordDTO) {
        let time_sec = (record.hw_ts_us - self.base_time_us) as f64 / 1_000_000.0;
        let can_id = record.frame.can_id;
        
        // Plot first data byte as value (or 0 if empty)
        let value = hex::decode(&record.frame.data_hex)
            .ok()
            .and_then(|bytes| bytes.first().copied())
            .unwrap_or(0) as f64;

        self.plot_data
            .entry(can_id)
            .or_default()
            .push([time_sec, value]);
    }

    fn clear(&mut self) {
        self.plot_data.clear();
        self.current_idx = 0;
    }

    fn step(&mut self) {
        if self.current_idx < self.records.len() {
            let record = self.records[self.current_idx].clone();
            self.add_frame(&record);
            self.current_idx += 1;
        }
    }

    fn step_many(&mut self, count: usize) {
        for _ in 0..count {
            self.step();
        }
    }
}

impl eframe::App for PlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-advance frames when playing
        if self.is_playing && self.current_idx < self.records.len() {
            let frames_per_update = (10.0 * self.playback_speed) as usize;
            self.step_many(frames_per_update.max(1));
            ctx.request_repaint();
        }

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("⏮ Reset").clicked() {
                    self.clear();
                }

                if self.is_playing {
                    if ui.button("⏸ Pause").clicked() {
                        self.is_playing = false;
                    }
                } else {
                    if ui.button("▶ Play").clicked() {
                        self.is_playing = true;
                    }
                }

                if ui.button("⏭ Step").clicked() {
                    self.step_many(100);
                }

                ui.separator();

                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.playback_speed, 0.1..=10.0).logarithmic(true));

                ui.separator();

                ui.checkbox(&mut self.autoscroll, "Autoscroll");

                ui.separator();

                ui.label(format!(
                    "Frame {}/{} ({:.1}%)",
                    self.current_idx,
                    self.records.len(),
                    100.0 * self.current_idx as f64 / self.records.len().max(1) as f64
                ));

                ui.separator();
                
                // Debug: show data point count
                let total_points: usize = self.plot_data.values().map(|v| v.len()).sum();
                ui.label(format!("IDs: {} | Points: {}", self.plot_data.len(), total_points));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let plot = Plot::new("can_plot")
                .legend(Legend::default())
                .show_axes(true)
                .show_grid(true)
;

            let plot = if self.autoscroll && !self.plot_data.is_empty() {
                // Find max time and set view to show last 10 seconds
                let max_time = self.plot_data
                    .values()
                    .flat_map(|pts| pts.last())
                    .map(|p| p[0])
                    .fold(0.0f64, |a, b| a.max(b));
                
                let view_width = 10.0;
                let x_min = (max_time - view_width).max(0.0);
                plot.include_x(x_min).include_x(max_time + 0.5)
            } else {
                plot
            };

            plot.show(ui, |plot_ui| {
                // Sort CAN IDs for consistent colors
                let mut can_ids: Vec<_> = self.plot_data.keys().copied().collect();
                can_ids.sort();

                for can_id in can_ids {
                    if let Some(points) = self.plot_data.get(&can_id) {
                        if !points.is_empty() {
                            let line = Line::new(PlotPoints::from_iter(points.iter().copied()))
                                .name(format!("0x{:03X}", can_id))
                                .width(2.0);
                            plot_ui.line(line);
                        }
                    }
                }
            });
        });
    }
}

pub fn run_gui(records: Vec<CANFrameRecordDTO>) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 600.0])
            .with_title("Nestor Replay - CAN Frame Visualization"),
        ..Default::default()
    };

    eframe::run_native(
        "Nestor Replay",
        options,
        Box::new(|_cc| Ok(Box::new(PlotApp::new(records)))),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {}", e))
}
