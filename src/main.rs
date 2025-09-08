// main.rs - Async Conway's Game of Life with Row Coroutines
// Uses existing patterns.rs and ui.rs modules

use eframe::egui;
use egui::Color32;
use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::{RwLock, Notify};

mod ui;        // Your existing ui.rs module
mod patterns;  // Your existing patterns.rs module

type TGrid = [[bool; 52]; 52];

// Cargo.toml dependencies needed:
// [dependencies]
// eframe = "0.24"
// egui = "0.24"
// tokio = { version = "1.0", features = ["full"] }

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
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

/// Row coroutine that processes cells cooperatively
async fn row_coroutine(
    row_index: usize,
    current_grid: Arc<RwLock<TGrid>>,
    next_grid: Arc<RwLock<TGrid>>,
    generation_sync: Arc<Notify>,
) {
    loop {
        // Process all cells in this row
        for col in 1..=50 {
            // Read current grid state
            let grid = current_grid.read().await;
            
            // Count neighbors using baked-in coordinates (same logic as original)
            let mut count = 0;
            let neighbors = [
                (row_index-1, col-1), (row_index-1, col), (row_index-1, col+1),
                (row_index, col-1),                        (row_index, col+1),
                (row_index+1, col-1), (row_index+1, col), (row_index+1, col+1)
            ];
            
            for &(nr, nc) in &neighbors {
                if grid[nr][nc] { count += 1; }
            }
            
            let current_alive = grid[row_index][col];
            drop(grid); // Release read lock
            
            // Apply Conway's rules (same as original)
            let next_state = match (current_alive, count) {
                (true, 2) | (true, 3) => true,   // Survival
                (false, 3)            => true,   // Birth
                _                     => false,  // Death or stays dead
            };
            
            // Write to next grid
            {
                let mut next = next_grid.write().await;
                next[row_index][col] = next_state;
            }
            
            // Yield after each cell for cooperative scheduling
            tokio::task::yield_now().await;
        }
        
        // Wait for all rows to complete before next generation
        generation_sync.notified().await;
    }
}

/// Factory function that creates and spawns row coroutines
fn spawn_row_coroutine(
    row_index: usize,
    current_grid: Arc<RwLock<TGrid>>,
    next_grid: Arc<RwLock<TGrid>>,
    generation_sync: Arc<Notify>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(row_coroutine(row_index, current_grid, next_grid, generation_sync))
}

/// Async Conway's Game of Life - maintains same interface as original for UI compatibility
pub struct GameOfLife {
    // Grid state - now async-shared
    current_grid: Arc<RwLock<TGrid>>,
    next_grid: Arc<RwLock<TGrid>>,
    
    // Keep same fields as original for UI compatibility
    pub grid: TGrid,  // Cached copy for UI rendering
    pub is_running: bool,
    pub last_update: Instant,
    pub update_interval: Duration,
    pub generation: u32,
    pub live_color: Color32,
    pub dead_color: Color32,
    pub selected_pattern: usize,
    
    // Async runtime components
    generation_sync: Arc<Notify>,
    runtime: Option<tokio::runtime::Handle>,
    
    // Cycle detection (same as original)
    grid_history: [u64; 10],
    history_count: usize,
}

impl Default for GameOfLife {
    fn default() -> Self {
        let current_grid = Arc::new(RwLock::new([[false; 52]; 52]));
        let next_grid = Arc::new(RwLock::new([[false; 52]; 52]));
        let generation_sync = Arc::new(Notify::new());
        
        // Spawn 50 row coroutines (one per row from 1 to 50)
        for row in 1..=50 {
            spawn_row_coroutine(
                row,
                current_grid.clone(),
                next_grid.clone(),
                generation_sync.clone(),
            );
        }
        
        Self {
            current_grid,
            next_grid,
            grid: [[false; 52]; 52],  // Cached copy for UI
            is_running: false,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(200),
            generation: 0,
            live_color: Color32::from_rgb(0, 200, 0),
            dead_color: Color32::from_rgb(40, 40, 40),
            selected_pattern: 0,
            generation_sync,
            runtime: None,
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
        // Initialize runtime handle on first call
        if self.runtime.is_none() {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                self.runtime = Some(handle);
            } else {
                // Fallback: create a new runtime if none exists
                eprintln!("Warning: No tokio runtime found, async features disabled");
                return;
            }
        }
        
        if let Some(handle) = &self.runtime {
            // Use block_on to bridge async/sync gap for eframe
            handle.block_on(async {
                // Clear the next grid first
                {
                    let mut next = self.next_grid.write().await;
                    *next = [[false; 52]; 52];
                }
                
                // Signal all row coroutines to start processing this generation
                self.generation_sync.notify_waiters();
                
                // Give coroutines time to process their rows
                // Since calculations are much faster than rendering, this should be quick
                tokio::time::sleep(Duration::from_millis(1)).await;
                
                // Swap grids (next becomes current)
                {
                    let mut current = self.current_grid.write().await;
                    let mut next = self.next_grid.write().await;
                    std::mem::swap(&mut *current, &mut *next);
                    
                    // Update cached copy for UI rendering
                    self.grid = *current;
                }
                
                self.generation += 1;
            });
            
            // Check for cycles and pause if detected (same as original)
            if self.check_for_cycle() {
                self.is_running = false;
            }
        }
    }
    
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
        if self.grid_history.contains(&current_hash) {
            return true; // Cycle detected
        }
        
        self.grid_history[self.history_count % 10] = current_hash; // Circular buffer
        self.history_count += 1;
        false // No cycle
    }
    
    fn clear_grid(&mut self) {
        if let Some(handle) = &self.runtime {
            handle.block_on(async {
                let mut current = self.current_grid.write().await;
                *current = [[false; 52]; 52];
                self.grid = *current; // Update cached copy
            });
        } else {
            self.grid = [[false; 52]; 52];
        }
        self.generation = 0;
        self.grid_history = [0; 10];  // Reset array to zeros
        self.history_count = 0;       // Reset counter
    }
    
    fn apply_selected_pattern(&mut self) {
        if let Some(pattern) = patterns::PATTERNS.get(self.selected_pattern) {
            if let Some(handle) = &self.runtime {
                handle.block_on(async {
                    let mut current = self.current_grid.write().await;
                    patterns::apply_pattern(&mut *current, pattern);
                    self.grid = *current; // Update cached copy
                });
            } else {
                patterns::apply_pattern(&mut self.grid, pattern);
            }
            self.generation = 0;
            self.grid_history = [0; 10];  // Reset array to zeros
            self.history_count = 0;       // Reset counter
        }
    }
    
    fn check_border_cells_dead(&self) -> bool {
        // Check cached grid copy (same logic as original)
        for i in 0..52 {
            if self.grid[0][i]  != false { panic!("Top border cell [0, {}] should be false", i); }
            if self.grid[51][i] != false { panic!("Bottom border cell [51, {}] should be false", i); }
            if self.grid[i][0]  != false { panic!("Left border cell [{}, 0] should be false", i); }
            if self.grid[i][51] != false { panic!("Right border cell [{}, 51] should be false", i); }
        }
        true
    }
}

// Additional async-specific methods for pattern operations
impl GameOfLife {
    pub fn apply_random_pattern_async(&mut self) {
        if let Some(handle) = &self.runtime {
            handle.block_on(async {
                let mut current = self.current_grid.write().await;
                patterns::apply_random_pattern(&mut *current, self.generation);
                self.grid = *current; // Update cached copy
            });
        } else {
            patterns::apply_random_pattern(&mut self.grid, self.generation);
        }
        self.generation = 0;
        self.grid_history = [0; 10];
        self.history_count = 0;
    }
    
    pub fn toggle_cell_async(&mut self, row: usize, col: usize) {
        if row >= 1 && row <= 50 && col >= 1 && col <= 50 {
            if let Some(handle) = &self.runtime {
                handle.block_on(async {
                    let mut current = self.current_grid.write().await;
                    current[row][col] = !current[row][col];
                    self.grid = *current; // Update cached copy
                });
            } else {
                self.grid[row][col] = !self.grid[row][col];
            }
        }
    }
}