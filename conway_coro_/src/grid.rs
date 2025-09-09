// grid.rs - Grid types for Conway's Game of Life

// Compile-time grid size configuration
pub const GRID_SIZE: usize = 50;                      // Active grid size (50x50 playing area)
pub const TOTAL_SIZE: usize = GRID_SIZE + 2;          // Total size including borders
pub const GRID_START: usize = 1;                      // Start of active area  
pub const GRID_END: usize = GRID_SIZE + 1;            // End of active area (1..GRID_SIZE+1)

pub type TRow = [bool; TOTAL_SIZE];
pub type TGrid = [TRow; TOTAL_SIZE];