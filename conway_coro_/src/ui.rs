// ui.rs - Modified to work with async version
// Only minimal changes to support async random pattern and cell toggling

use eframe::egui;
use egui::{Color32, Rect, Stroke, Vec2};
use std::time::{Duration, Instant};
use crate::{GameOfLife, patterns, GameOfLifeInterface};

impl eframe::App for GameOfLife {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-update if running
        if self.is_running && self.last_update.elapsed() >= self.update_interval {
            self.update_generation();
            self.last_update = Instant::now();
            ctx.request_repaint(); // Ensure continuous updates
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Async Conway's Game of Life (Row Coroutines)");
            
            // Controls
            ui.horizontal(|ui| {
                let button_text = if self.is_running { "⏸ Pause" } else { "▶ Start" };
                if ui.button(button_text).clicked() {
                    self.is_running = !self.is_running;
                    if self.is_running {
                        self.last_update = Instant::now();
                    }
                }
                
                if ui.button("⏹ Clear").clicked() {
                    self.is_running = false;
                    self.clear_grid();
                }
                
                if ui.button("🎲 Random").clicked() {
                    self.is_running = false;
                    self.apply_random_pattern_async(); // Use async version
                }
                
                ui.separator();
                
                // Pattern dropdown
                ui.label("Pattern:");
                egui::ComboBox::from_id_source("pattern_selector")
                    .selected_text(patterns::PATTERNS[self.selected_pattern].name)
                    .show_ui(ui, |ui| {
                        for (i, pattern) in patterns::PATTERNS.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_pattern, i, pattern.name);
                        }
                    });
                
                if ui.button("Apply Pattern").clicked() {
                    self.is_running = false;
                    self.apply_selected_pattern();
                }
                
                ui.separator();
                
                ui.label(format!("Generation: {}", self.generation));
            });
            
            ui.separator();
            
            // Speed control
            ui.horizontal(|ui| {
                ui.label("Speed:");
                let mut speed = 1000.0 / self.update_interval.as_millis() as f32;
                if ui.add(egui::Slider::new(&mut speed, 0.5..=90.0).suffix(" gen/sec")).changed() {
                    self.update_interval = Duration::from_millis((1000.0 / speed) as u64);
                }
                
                ui.separator();
                
                // Show current colors
                ui.label("Live:");
                ui.color_edit_button_srgba(&mut self.live_color);
                ui.label("Dead:");
                ui.color_edit_button_srgba(&mut self.dead_color);
            });
            
            ui.separator();
            
            // Instructions - updated to mention async coroutines
            ui.label("🚀 Each row runs as an async coroutine that yields cooperatively!");
            ui.label("Click cells to toggle them alive/dead. Use Start/Pause to run the simulation.");
            
            ui.separator();
            
            // Draw the grid (only show the active 50x50 area)
            let box_size = 15.0;
            let spacing = 0.5;
            let grid_size = 50;  // Display size stays 50x50
            
            let start_pos = ui.cursor().min;
            let total_size = Vec2::splat((box_size + spacing) * grid_size as f32 - spacing);
            
            let (response, painter) = ui.allocate_painter(total_size, egui::Sense::click());
            
            // Fill background
            painter.rect_filled(
                Rect::from_min_size(start_pos, total_size),
                0.0,
                Color32::BLACK,
            );
            
            // Draw only the active area (grid[1..51][1..51])
            for display_row in 0..grid_size {
                for display_col in 0..grid_size {
                    let grid_row = display_row + 1;  // Map to grid[1..51]
                    let grid_col = display_col + 1;  // Map to grid[1..51]
                    
                    let x = start_pos.x + display_col as f32 * (box_size + spacing);
                    let y = start_pos.y + display_row as f32 * (box_size + spacing);
                    
                    let rect = Rect::from_min_size(
                        egui::pos2(x, y),
                        Vec2::splat(box_size),
                    );
                    
                    // Choose color based on cell state
                    let cell_color = if self.grid[grid_row][grid_col] {
                        self.live_color
                    } else {
                        self.dead_color
                    };
                    
                    painter.rect_filled(rect, 1.0, cell_color);
                    
                    // Draw subtle border
                    painter.rect_stroke(rect, 1.0, Stroke::new(0.2, Color32::from_gray(60)));
                    
                    // Handle clicking (only when not running) - use async version
                    if !self.is_running && response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            if rect.contains(pos) {
                                self.toggle_cell_async(grid_row, grid_col); // Use async version
                            }
                        }
                    }
                }
            }
            
            ui.separator();
            
            // Statistics (count only the active area)
            let live_cells: usize = (1..51).map(|row| 
                (1..51).filter(|&col| self.grid[row][col]).count()
            ).sum();
            
            ui.horizontal(|ui| {
                ui.label(format!("Live cells: {}", live_cells));
                ui.label(format!("Dead cells: {}", 2500 - live_cells));
                ui.label(format!("Population: {:.1}%", (live_cells as f32 / 2500.0) * 100.0));
            });
        });
        
        // Request repaint if running to keep animation smooth
        if self.is_running {
            ctx.request_repaint();
        }
    }
}