// main.rs - Async Conway's Game of Life with Row Coroutines
// Uses existing patterns.rs and ui.rs modules

use eframe::egui;
use egui::Color32;
use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

mod grid;      // Grid types
mod ui;        // Your existing ui.rs module
mod patterns;  // Your existing patterns.rs module

use grid::TGrid;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 950.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Async Conway's Game of Life",
        options,
        Box::new(|_cc| Box::new(GameOfLife::default())),
    )
}

/// Row coroutine function that processes a specific row
async fn process_row(row_index: usize, current_grid: TGrid) -> (usize, [bool; 52]) {
    let mut row_result = [false; 52];
    for col in 1..51 {
        let mut count = 0;
        // Neighbor positions for this specific row
        let neighbors = [
            (row_index-1,col-1),(row_index-1,col),(row_index-1,col+1),(row_index,col-1),                    
            (row_index+1,col-1),(row_index+1,col),(row_index+1,col+1),(row_index,col+1)
        ];
        
        for &(nr, nc) in &neighbors {
            if current_grid[nr][nc] { count += 1; }
        }
        
        let current_alive = current_grid[row_index][col];
        
        let next_state = match (current_alive, count) {
            (true, 2) | (true, 3) => true,   // Survival
            (false, 3)            => true,   // Birth
            _                     => false,  // Death or stays dead
        };
        
        row_result[col] = next_state;
        
        tokio::task::yield_now().await;  // Cooperative yielding!
    }
    (row_index, row_result)  // Return (row_id, completed_row)
}

/// Async Conway's Game of Life - simplified with plain grids
pub struct GameOfLife {
    current_grid: TGrid,
    next_grid: TGrid,
    
    pub grid: TGrid,  // Cached copy for UI rendering
    pub is_running: bool,
    pub last_update: Instant,
    pub update_interval: Duration,
    pub generation: u32,
    pub live_color: Color32,
    pub dead_color: Color32,
    pub selected_pattern: usize,
    
    runtime: tokio::runtime::Runtime,
    
    grid_history: [u64; 10],
    history_count: usize,
}

impl Default for GameOfLife {
    fn default() -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        
        Self {
            current_grid: [[false; 52]; 52],
            next_grid: [[false; 52]; 52],
            grid: [[false; 52]; 52],
            is_running: false,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(200),
            generation: 0,
            live_color: Color32::from_rgb(0, 200, 0),
            dead_color: Color32::from_rgb(40, 40, 40),
            selected_pattern: 0,
            runtime,
            grid_history: [0; 10],
            history_count: 0,
        }
    }
}

/// Same interface as original - required by existing UI module
pub trait GameOfLifeInterface {
    fn update_generation(&mut self);
    fn hash_grid(&self) -> u64;
    fn check_for_cycle(&mut self) -> bool;
    fn clear_grid(&mut self);
    fn apply_selected_pattern(&mut self);
    fn check_border_cells_dead(&self) -> bool;
}

impl GameOfLifeInterface for GameOfLife {
    fn update_generation(&mut self) {
        self.runtime.block_on(async {
            self.next_grid = [[false; 52]; 52];
            
            // Copy the current grid to avoid borrowing issues with 'static
            let grid_copy = self.current_grid;
            
            // Spawn all 50 row coroutines simultaneously for time-slicing
            let mut handles = Vec::new();
            for row in 1..51 {
                let handle = tokio::spawn(process_row(row, grid_copy));
                handles.push(handle);
            }
            
            // Wait for all coroutines and collect results with row identification
            for handle in handles {
                let (row_index, completed_row) = handle.await.unwrap();
                self.next_grid[row_index] = completed_row;
            }
            
            std::mem::swap(&mut self.current_grid, &mut self.next_grid);
            self.grid = self.current_grid;
            self.generation += 1;
        });
        
        if self.check_for_cycle() { self.is_running = false; }
    }
    
    fn hash_grid(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for row in 1..51 {
            for col in 1..51 {
                self.grid[row][col].hash(&mut hasher);
            }
        }
        hasher.finish()
    }
    
    fn check_for_cycle(&mut self) -> bool {
        let current_hash = self.hash_grid();
        if self.grid_history.contains(&current_hash) { return true; }
        self.grid_history[self.history_count % 10] = current_hash;
        self.history_count += 1;
        false
    }
    
    fn clear_grid(&mut self) {
        self.current_grid = [[false; 52]; 52];
        self.grid = self.current_grid;
        self.generation = 0;
        self.grid_history = [0; 10];
        self.history_count = 0;
    }
    
    fn apply_selected_pattern(&mut self) {
        if let Some(pattern) = patterns::PATTERNS.get(self.selected_pattern) {
            patterns::apply_pattern(&mut self.current_grid, pattern);
            self.grid = self.current_grid;
            self.generation = 0;
            self.grid_history = [0; 10];
            self.history_count = 0;
        }
    }
    
    fn check_border_cells_dead(&self) -> bool {
        for i in 0..52 {
            if self.grid[0][i]  != false { panic!("Top border cell [0, {}] should be false", i); }
            if self.grid[51][i] != false { panic!("Bottom border cell [51, {}] should be false", i); }
            if self.grid[i][0]  != false { panic!("Left border cell [{}, 0] should be false", i); }
            if self.grid[i][51] != false { panic!("Right border cell [{}, 51] should be false", i); }
        }
        true
    }
}

impl GameOfLife {
    pub fn apply_random_pattern_async(&mut self) {
        patterns::apply_random_pattern(&mut self.current_grid, self.generation);
        self.grid = self.current_grid;
        self.generation = 0;
        self.grid_history = [0; 10];
        self.history_count = 0;
    }
    
    pub fn toggle_cell_async(&mut self, row: usize, col: usize) {
        if row >= 1 && row < 51 && col >= 1 && col < 51 {
            self.current_grid[row][col] = !self.current_grid[row][col];
            self.grid = self.current_grid;
        }
    }
}