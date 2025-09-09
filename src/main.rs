// main.rs - Time-Sliced Async Conway's Game of Life with Row Coroutines
// Uses existing patterns.rs and ui.rs modules

use eframe::egui;
use egui::Color32;
use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

mod grid;      // Grid types
mod ui;        // Your existing ui.rs module
mod patterns;  // Your existing patterns.rs module

use grid::{TGrid, GRID_START, GRID_END, TOTAL_SIZE};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 950.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Time-Sliced Conway's Game of Life",
        options,
        Box::new(|_cc| Box::new(GameOfLife::default())),
    )
}

/// Factory function that creates time-sliced row coroutine closures
fn create_time_sliced_row_coroutine(row_index: usize) -> impl FnMut(TGrid, Duration) -> std::pin::Pin<Box<dyn std::future::Future<Output = (bool, [bool; TOTAL_SIZE])>>> {
    let mut current_col = GRID_START;
    //let mut completed = false;
    let mut result = [false; TOTAL_SIZE];
    
    move |current_grid: TGrid, time_budget: Duration| {
        Box::pin(async move {
            //if completed {
            //    return (true, result);
            //}
            
            let start = Instant::now();
            
            while current_col < GRID_END {
                // Check if time budget is exhausted
                if start.elapsed() >= time_budget {
                    break;  // Time's up, exit and yield control
                }
                
                let col = current_col;
                let mut count = 0;
                
                // Baked-in neighbor positions for this specific row
                let neighbors = [
                    (row_index-1,col-1),(row_index-1,col),(row_index-1,col+1),
                    (row_index,col-1),                    (row_index,col+1),
                    (row_index+1,col-1),(row_index+1,col),(row_index+1,col+1)
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
                
                result[col] = next_state;
                current_col += 1;
            }
            
            // Check if row is complete
            if current_col >= GRID_END {
                //completed = true;
                (true, result)
            } else {
                (false, result)
            }
        })
    }
}

/// Generation processor that manages time-sliced closure-based coroutines
struct GenerationProcessor {
    row_coroutines: Vec<Box<dyn FnMut(TGrid, Duration) -> std::pin::Pin<Box<dyn std::future::Future<Output = (bool, [bool; TOTAL_SIZE])>>>>>,
    time_budget_per_slice: Duration,
}

impl GenerationProcessor {
    fn new(time_budget_per_slice: Duration) -> Self {
        let mut row_coroutines = Vec::new();
        
        // Create coroutines for active rows only (GRID_START..GRID_END)
        for row in GRID_START..GRID_END {
            let coroutine = create_time_sliced_row_coroutine(row);
            row_coroutines.push(Box::new(coroutine) as Box<dyn FnMut(TGrid, Duration) -> std::pin::Pin<Box<dyn std::future::Future<Output = (bool, [bool; TOTAL_SIZE])>>>>);
        }
        
        Self {
            row_coroutines,
            time_budget_per_slice,
        }
    }
    
    async fn process_generation(&mut self, current_grid: TGrid) -> TGrid {
        let active_rows = GRID_END - GRID_START;  // Should be GRID_SIZE
        let mut completed_rows = vec![false; active_rows];  // Track which rows are done
        let mut results = vec![[false; TOTAL_SIZE]; active_rows];    // Store completed row results
        
        // Keep giving time slices until all rows complete
        while !completed_rows.iter().all(|&done| done) {
            for (i, row_coroutine) in self.row_coroutines.iter_mut().enumerate() {
                if !completed_rows[i] {
                    let (is_complete, row_result) = row_coroutine(current_grid, self.time_budget_per_slice).await;
                    
                    if is_complete {
                        completed_rows[i] = true;
                        results[i] = row_result;
                    }
                }
            }
        }
        
        // Collect results into new grid
        self.collect_results(results)
    }
    
    fn collect_results(&self, results: Vec<[bool; TOTAL_SIZE]>) -> TGrid {
        let mut next_grid = [[false; TOTAL_SIZE]; TOTAL_SIZE];
        for (i, row_result) in results.iter().enumerate() {
            let row_index = i + GRID_START;  // Map back to active range (GRID_START..GRID_END)
            next_grid[row_index] = *row_result;
        }
        next_grid
    }
    
    fn set_time_budget(&mut self, new_budget: Duration) {
        self.time_budget_per_slice = new_budget;
    }
}

/// Time-Sliced Conway's Game of Life
pub struct GameOfLife {
    current_grid: TGrid,
    
    pub grid: TGrid,  // Cached copy for UI rendering
    pub is_running: bool,
    pub last_update: Instant,
    pub update_interval: Duration,
    pub generation: u32,
    pub live_color: Color32,
    pub dead_color: Color32,
    pub selected_pattern: usize,
    
    runtime: tokio::runtime::Runtime,
    generation_processor: GenerationProcessor,
    
    // Cycle detection
    grid_history: [u64; 10],
    history_count: usize,
    
    // Time slice control
    pub time_slice_ms: f32,  // Exposed for UI control
}

impl Default for GameOfLife {
    fn default() -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let time_slice_ms = 2.0;  // 2ms default time slices
        let generation_processor = GenerationProcessor::new(Duration::from_millis(time_slice_ms as u64));
        
        Self {
            current_grid: [[false; TOTAL_SIZE]; TOTAL_SIZE],
            grid: [[false; TOTAL_SIZE]; TOTAL_SIZE],
            is_running: false,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(200),
            generation: 0,
            live_color: Color32::from_rgb(0, 200, 0),
            dead_color: Color32::from_rgb(40, 40, 40),
            selected_pattern: 0,
            runtime,
            generation_processor,
            grid_history: [0; 10],
            history_count: 0,
            time_slice_ms,
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
        // Update time slice if changed
        let time_budget = Duration::from_millis(self.time_slice_ms as u64);
        self.generation_processor.set_time_budget(time_budget);
        
        self.runtime.block_on(async {
            // Process generation with time-sliced coroutines
            let next_grid = self.generation_processor.process_generation(self.current_grid).await;
            
            self.current_grid = next_grid;
            self.grid = self.current_grid;
            self.generation += 1;
        });
        
        if self.check_for_cycle() { self.is_running = false; }
    }
    
    fn hash_grid(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for row in GRID_START..GRID_END {
            for col in GRID_START..GRID_END {
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
        self.current_grid = [[false; TOTAL_SIZE]; TOTAL_SIZE];
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
        for i in 0..TOTAL_SIZE {
            if self.grid[0][i] != false { panic!("Top border cell [0, {}] should be false", i); }
            if self.grid[TOTAL_SIZE-1][i] != false { panic!("Bottom border cell [{}, {}] should be false", TOTAL_SIZE-1, i); }
            if self.grid[i][0] != false { panic!("Left border cell [{}, 0] should be false", i); }
            if self.grid[i][TOTAL_SIZE-1] != false { panic!("Right border cell [{}, {}] should be false", i, TOTAL_SIZE-1); }
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
        if row >= GRID_START && row < GRID_END && col >= GRID_START && col < GRID_END {
            self.current_grid[row][col] = !self.current_grid[row][col];
            self.grid = self.current_grid;
        }
    }
}