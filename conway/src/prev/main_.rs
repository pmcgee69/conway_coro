use eframe::egui;
use egui::Color32;
use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

mod ui;
mod patterns;

type TGrid = [[bool; 52]; 52];
type CellFn = dyn Fn(&TGrid, &mut TGrid);
type SmartCellFn = Box<CellFn>;

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
    grid             : TGrid,              // 0-51 grid with border
    next_grid        : TGrid,
    cell_functions   : Vec<SmartCellFn>,
    is_running       : bool,
    last_update      : Instant,
    update_interval  : Duration,
    generation       : u32,
    live_color       : Color32,
    dead_color       : Color32,
    selected_pattern : usize,
    grid_history     : [u64; 10],         // Fixed array of 10 grid hashes
    history_count    : usize,             // Counter for circular buffer
}

impl Default for GameOfLife {
    fn default() -> Self {
        // Function factory that creates specialized functions
        fn create_cell_fn(row: usize, col: usize) -> impl Fn(&TGrid, &mut TGrid) {
            move |current_grid: &TGrid, next_grid: &mut TGrid| {
                // Count live neighbors using baked-in coordinates
                let mut count = 0;
                
                // Baked-in neighbor positions for this specific cell
                let neighbors = [
                    (row-1,col-1),(row-1,col),(row-1,col+1),(row,col-1),                    
                    (row+1,col-1),(row+1,col),(row+1,col+1),(row,col+1)
                ];
                
                for &(nr, nc) in &neighbors {  if current_grid[nr][nc] {count+=1;} }
                
                // Apply Conway's rules and write directly to next_grid
                next_grid[row][col] = match (current_grid[row][col], count) {
                    (true, 2) | (true, 3) => true,   // Survival
                    (false, 3)            => true,   // Birth
                    _                     => false,  // Death or stays dead
                };
            }
        }
        
        // Create a single vector of 2500 lambda functions
        let mut cell_functions = Vec::with_capacity(2500);
        
        // Initialize functions with coordinates from (1,1) to (50,50)
        for grid_row in 1..51 {
            for grid_col in 1..51 {
                cell_functions.push(Box::new(create_cell_fn(grid_row, grid_col)) as SmartCellFn);
            }
        }
        
        Self {
            grid             : [[false; 52]; 52],
            next_grid        : [[false; 52]; 52],
            cell_functions,
            is_running       : false,
            last_update      : Instant::now(),
            update_interval  : Duration::from_millis(200),
            generation       : 0,
            live_color       : Color32::from_rgb(0, 200, 0),
            dead_color       : Color32::from_rgb(40, 40, 40),
            selected_pattern : 0,
            grid_history     : [0; 10],  // Initialize array with zeros
            history_count    : 0,        // Start counter at zero
        }
    }
}

impl GameOfLife {
    fn hash_grid(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        // Only hash the active area (1..51, 1..51), ignore borders
        for row in 1..51 {
            for col in 1..51 {
                self.grid[row][col].hash(&mut hasher);
            }
        }
        hasher.finish()
    }
    
    fn check_for_cycle(&mut self) -> bool {
        let current_hash = self.hash_grid();
        if self.grid_history.contains(&current_hash) { return true; } // Cycle detected
        
        self.grid_history[self.history_count % 10] = current_hash; // Circular buffer
        self.history_count += 1;
        
        false // No cycle
    }
    
    fn check_border_cells_dead(&self) -> bool {
        for i in 0..52 {
            if self.next_grid[0][i]  != false { panic!("Top border cell [0, {}] should be false", i); }
            if self.next_grid[51][i] != false { panic!("Bottom border cell [51, {}] should be false", i); }
            if self.next_grid[i][0]  != false { panic!("Left border cell [{}, 0] should be false", i); }
            if self.next_grid[i][51] != false { panic!("Right border cell [{}, 51] should be false", i); }
        }
        true
    }
    
    fn update_generation(&mut self) {
        // Clear the next grid first
        self.next_grid = [[false; 52]; 52];
        
        // Execute all 2500 cell functions, each writes directly to next_grid
        self.cell_functions.iter().for_each(|cell_func| {
            cell_func(&self.grid, &mut self.next_grid);
        });
        
        // Assert that border cells remain dead (error checking) - debug only
        debug_assert!(self.check_border_cells_dead());
        
        // Copy next generation to current
        self.grid = self.next_grid;
        self.generation += 1;
        
        // Check for cycles and pause if detected
        if self.check_for_cycle() { self.is_running = false; }
    }
    
    fn clear_grid(&mut self) {
        self.grid = [[false; 52]; 52];
        self.generation = 0;
        self.grid_history = [0; 10];  // Reset array to zeros
        self.history_count = 0;       // Reset counter
    }
    
    fn apply_selected_pattern(&mut self) {
        if let Some(pattern) = patterns::PATTERNS.get(self.selected_pattern) {
            patterns::apply_pattern(&mut self.grid, pattern);
            self.generation = 0;
            self.grid_history = [0; 10];  // Reset array to zeros
            self.history_count = 0;       // Reset counter
        }
    }
}
