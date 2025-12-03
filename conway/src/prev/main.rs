use eframe::egui;
use egui::Color32;
use std::time::{Duration, Instant};

mod ui;
mod patterns;

type TGrid = [[bool; 52]; 52];

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 950.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Conway's Game of Life",
        options,
        Box::new(|_cc| Box::new(GameOfLife::default())),
    )
}

struct GameOfLife {
    grid: TGrid,  // 0-51 grid with border
    next_grid: TGrid,
    cell_functions: Vec<Box<dyn Fn(&TGrid) -> bool>>,
    is_running: bool,
    last_update: Instant,
    update_interval: Duration,
    generation: u32,
    live_color: Color32,
    dead_color: Color32,
    show_color_picker: bool,
    picking_live: bool,
    selected_pattern: usize,
}

impl Default for GameOfLife {
    fn default() -> Self {
        // Function factory that creates specialized functions
        fn create_cell_function(row: usize, col: usize) -> impl Fn(&TGrid) -> bool {
            move |grid: &TGrid| {
                // Count live neighbors using baked-in coordinates
                let mut count = 0;
                
                // Baked-in neighbor positions for this specific cell
                let neighbors = [
                    (row - 1, col - 1), (row - 1, col), (row - 1, col + 1),
                    (row, col - 1),                     (row, col + 1),
                    (row + 1, col - 1), (row + 1, col), (row + 1, col + 1),
                ];
                
                for &(nr, nc) in &neighbors {
                    if grid[nr][nc] {
                        count += 1;
                    }
                }
                
                // Apply Conway's rules
                let is_alive = grid[row][col];
                match (is_alive, count) {
                    (true, 2) | (true, 3) => true,  // Survival
                    (false, 3) => true,             // Birth
                    _ => false,                     // Death or stays dead
                }
            }
        }
        
        // Create a single vector of 2500 lambda functions
        let mut cell_functions = Vec::with_capacity(2500);
        
        // Initialize functions with coordinates from (1,1) to (50,50)
        for grid_row in 1..51 {
            for grid_col in 1..51 {
                cell_functions.push(Box::new(create_cell_function(grid_row, grid_col)) as Box<dyn Fn(&TGrid) -> bool>);
            }
        }
        
        Self {
            grid: [[false; 52]; 52],
            next_grid: [[false; 52]; 52],
            cell_functions,
            is_running: false,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(200),
            generation: 0,
            live_color: Color32::from_rgb(0, 200, 0),
            dead_color: Color32::from_rgb(40, 40, 40),
            show_color_picker: false,
            picking_live: true,
            selected_pattern: 0,
        }
    }
}

impl GameOfLife {
    fn update_generation(&mut self) {
        // Execute all 2500 cell functions and collect results
        let results: Vec<bool> = self.cell_functions.iter()
            .map(|cell_func| cell_func(&self.grid))
            .collect();
        
        // Clear the next grid first
        self.next_grid = [[false; 52]; 52];
        
        // Set the results back into next_grid
        let mut index = 0;
        for grid_row in 1..51 {
            for grid_col in 1..51 {
                self.next_grid[grid_row][grid_col] = results[index];
                index += 1;
            }
        }
        
        // Assert that border cells remain dead (error checking)
        for i in 0..52 {
            assert_eq!(self.next_grid[0][i], false, "Top border cell [{}, {}] should be false", 0, i);
            assert_eq!(self.next_grid[51][i], false, "Bottom border cell [{}, {}] should be false", 51, i);
            assert_eq!(self.next_grid[i][0], false, "Left border cell [{}, {}] should be false", i, 0);
            assert_eq!(self.next_grid[i][51], false, "Right border cell [{}, {}] should be false", i, 51);
        }
        
        // Copy next generation to current
        self.grid = self.next_grid;
        self.generation += 1;
    }
    
    fn clear_grid(&mut self) {
        self.grid = [[false; 52]; 52];
        self.generation = 0;
    }
    
    fn random_pattern(&mut self) {
        patterns::apply_random_pattern(&mut self.grid, self.generation);
        self.generation = 0;
    }
    
    fn apply_selected_pattern(&mut self) {
        if let Some(pattern) = patterns::PATTERNS.get(self.selected_pattern) {
            patterns::apply_pattern(&mut self.grid, pattern);
            self.generation = 0;
        }
    }
}