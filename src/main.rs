// main.rs - Async Conway's Game of Life with Row Coroutines
// Uses existing patterns.rs and ui.rs modules

use eframe::egui;
use egui::Color32;
use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

mod ui;        // Your existing ui.rs module
mod patterns;  // Your existing patterns.rs module

type TRow = [bool; 52];
type TGrid = [TRow; 52];

// Union to view grid as either rows or flat vector
#[repr(C)]
union GridUnion {
    as_rows: TGrid,
    as_vector: [bool; 52 * 52],  // 2704 elements
}

impl GridUnion {
    fn new() -> Self {
        GridUnion {
            as_rows: [[false; 52]; 52]
        }
    }
    
    fn as_grid(&self) -> &TGrid {
        unsafe { &self.as_rows }
    }
    
    fn as_grid_mut(&mut self) -> &mut TGrid {
        unsafe { &mut self.as_rows }
    }
    
    fn as_vector_mut(&mut self) -> &mut [bool; 52 * 52] {
        unsafe { &mut self.as_vector }
    }
}

// Cargo.toml dependencies needed:
// [dependencies]
// eframe = "0.24"
// egui = "0.24"
// tokio = { version = "1.0", features = ["full"] }

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

/// Row coroutine that processes cells cooperatively - now lock-free!
async fn row_coroutine(
    row_index: usize,
    current_grid: &TGrid,           // Simple reference - no locking!
    my_row_slice: &mut [bool],      // Direct mutable slice - no Arc/Mutex!
) {
    // Process all cells in this row
    for col in 1..=50 {
        // Count neighbors using direct grid access
        let mut count = 0;
        let neighbors = [
            (row_index-1, col-1), (row_index-1, col), (row_index-1, col+1),
            (row_index, col-1),                        (row_index, col+1),
            (row_index+1, col-1), (row_index+1, col), (row_index+1, col+1)
        ];
        
        for &(nr, nc) in &neighbors {
            if current_grid[nr][nc] { count += 1; }
        }
        
        let current_alive = current_grid[row_index][col];
        
        // Apply Conway's rules
        let next_state = match (current_alive, count) {
            (true, 2) | (true, 3) => true,   // Survival
            (false, 3)            => true,   // Birth
            _                     => false,  // Death or stays dead
        };
        
        // Write directly to my exclusive row slice - no locking!
        my_row_slice[col] = next_state;
        
        // Yield after each cell for cooperative scheduling
        tokio::task::yield_now().await;
    }
    
    // No sync needed - coroutine completes and terminates
}

/// Async Conway's Game of Life - lock-free version
pub struct GameOfLife {
    // Grid state - no more Arc/RwLock/Mutex!
    current_grid: GridUnion,
    next_grid: GridUnion,
    
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
    runtime: tokio::runtime::Runtime,
    
    // Cycle detection (same as original)
    grid_history: [u64; 10],
    history_count: usize,
}

impl Default for GameOfLife {
    fn default() -> Self {
        // Create tokio runtime for coroutines
        let runtime = tokio::runtime::Runtime::new().unwrap();
        
        Self {
            current_grid: GridUnion::new(),
            next_grid: GridUnion::new(),
            grid: [[false; 52]; 52],  // Cached copy for UI
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
        // Use block_on to run async code in sync context (as you intended)
        self.runtime.block_on(async {
            // Clear the next grid first
            *self.next_grid.as_grid_mut() = [[false; 52]; 52];
            
            // Get the flat vector view and split it into non-overlapping slices
            let current_grid_ref = unsafe { &*(&self.current_grid.as_rows as *const TGrid) };
            let next_vector = self.next_grid.as_vector_mut();
            
            // Split the vector into 52 row slices (each 52 elements)
            let mut row_slices = Vec::new();
            for row in 0..52 {
                let start = row * 52;
                let end = start + 52;
                let row_slice = &mut next_vector[start..end];
                row_slices.push(row_slice as *mut [bool]);
            }
            
            // Spawn 50 row coroutines with direct slice access
            let mut handles = Vec::new();
            for row in 1..=50 {
                let my_row_slice = unsafe { &mut *row_slices[row] };
                
                let handle = tokio::spawn(row_coroutine(
                    row,
                    current_grid_ref,       // Direct reference - no Arc!
                    my_row_slice,           // Direct mutable slice - no Mutex!
                ));
                handles.push(handle);
            }
            
            // Wait for all row coroutines to complete
            for handle in handles {
                handle.await.unwrap();
            }
            
            // Swap grids - simple memory swap
            std::mem::swap(&mut self.current_grid, &mut self.next_grid);
            
            // Update cached copy for UI rendering
            self.grid = *self.current_grid.as_grid();
            
            self.generation += 1;
        });
        
        // Check for cycles and pause if detected (same as original)
        if self.check_for_cycle() {
            self.is_running = false;
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
        *self.current_grid.as_grid_mut() = [[false; 52]; 52];
        self.grid = *self.current_grid.as_grid(); // Update cached copy
        self.generation = 0;
        self.grid_history = [0; 10];  // Reset array to zeros
        self.history_count = 0;       // Reset counter
    }
    
    fn apply_selected_pattern(&mut self) {
        if let Some(pattern) = patterns::PATTERNS.get(self.selected_pattern) {
            patterns::apply_pattern(self.current_grid.as_grid_mut(), pattern);
            self.grid = *self.current_grid.as_grid(); // Update cached copy
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

// Additional async-specific methods for UI compatibility
impl GameOfLife {
    pub fn apply_random_pattern_async(&mut self) {
        patterns::apply_random_pattern(self.current_grid.as_grid_mut(), self.generation);
        self.grid = *self.current_grid.as_grid(); // Update cached copy
        self.generation = 0;
        self.grid_history = [0; 10];
        self.history_count = 0;
    }
    
    pub fn toggle_cell_async(&mut self, row: usize, col: usize) {
        if row >= 1 && row <= 50 && col >= 1 && col <= 50 {
            self.current_grid.as_grid_mut()[row][col] = !self.current_grid.as_grid()[row][col];
            self.grid = *self.current_grid.as_grid(); // Update cached copy
        }
    }
}