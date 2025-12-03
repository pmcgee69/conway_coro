
use eframe::egui;
use egui::{Color32, Rect, Stroke, Vec2};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 900.0]),  // Larger window for 50x50 grid
        ..Default::default()
    };
    
    eframe::run_native(
        "Grid Display",
        options,
        Box::new(|_cc| Box::new(GridApp::default())),
    )
}

struct GridApp {
    grid: [[bool; 50]; 50],  // 50x50 grid
    fg_color: Color32,
    bg_color: Color32,
    show_color_picker: bool,
    picking_fg: bool,
}

impl Default for GridApp {
    fn default() -> Self {
        Self {
            grid: [[true; 50]; 50], // Start with all boxes filled (black) - 50x50
            fg_color: Color32::BLACK,
            bg_color: Color32::WHITE,
            show_color_picker: false,
            picking_fg: true,
        }
    }
}

impl eframe::App for GridApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("50x50 Grid Display");
            
            ui.horizontal(|ui| {
                if ui.button("Choose Foreground Color").clicked() {
                    self.show_color_picker = true;
                    self.picking_fg = true;
                }
                
                if ui.button("Choose Background Color").clicked() {
                    self.show_color_picker = true;
                    self.picking_fg = false;
                }
                
                // Show current colors
                ui.label("FG:");
                ui.color_edit_button_srgba(&mut self.fg_color);
                ui.label("BG:");
                ui.color_edit_button_srgba(&mut self.bg_color);
            });
            
            ui.separator();
            
            // Color picker window
            if self.show_color_picker {
                let mut open = true;
                egui::Window::new("Color Picker")
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.label(if self.picking_fg { "Pick Foreground Color" } else { "Pick Background Color" });
                        
                        let color_to_edit = if self.picking_fg { &mut self.fg_color } else { &mut self.bg_color };
                        
                        ui.color_edit_button_srgba(color_to_edit);
                        
                        if ui.button("Done").clicked() {
                            self.show_color_picker = false;
                        }
                    });
                
                if !open {
                    self.show_color_picker = false;
                }
            }
            
            ui.separator();
            
            // Instructions
            ui.label("Click on boxes to toggle them on/off");
            
            ui.separator();
            
            // Draw the grid
            let box_size = 15.0;  // Smaller boxes for 50x50 grid
            let spacing = 0.5;
            let grid_size = 50;   // 50x50 grid
            
            let start_pos = ui.cursor().min;
            let total_size = Vec2::splat((box_size + spacing) * grid_size as f32 - spacing);
            
            let (response, painter) = ui.allocate_painter(total_size, egui::Sense::click());
            
            // Fill background
            painter.rect_filled(
                Rect::from_min_size(start_pos, total_size),
                0.0,
                self.bg_color,
            );
            
            for row in 0..grid_size {
                for col in 0..grid_size {
                    let x = start_pos.x + col as f32 * (box_size + spacing);
                    let y = start_pos.y + row as f32 * (box_size + spacing);
                    
                    let rect = Rect::from_min_size(
                        egui::pos2(x, y),
                        Vec2::splat(box_size),
                    );
                    
                    // Draw box if it's "on"
                    if self.grid[row][col] {
                        painter.rect_filled(rect, 0.0, self.fg_color);
                    }
                    
                    // Draw border
                    painter.rect_stroke(rect, 0.0, Stroke::new(0.3, Color32::GRAY));
                    
                    // Handle clicking
                    if response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            if rect.contains(pos) {
                                self.grid[row][col] = !self.grid[row][col];
                            }
                        }
                    }
                }
            }
            
            ui.separator();
            
            // Control buttons
            ui.horizontal(|ui| {
                if ui.button("Fill All").clicked() {
                    self.grid = [[true; 50]; 50];  // 50x50
                }
                
                if ui.button("Clear All").clicked() {
                    self.grid = [[false; 50]; 50]; // 50x50
                }
                
                if ui.button("Checkerboard").clicked() {
                    for row in 0..50 {  // 50x50
                        for col in 0..50 {
                            self.grid[row][col] = (row + col) % 2 == 0;
                        }
                    }
                }
            });
        });
    }
}