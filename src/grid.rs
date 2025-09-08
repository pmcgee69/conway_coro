// grid.rs - Grid types and union for Conway's Game of Life

pub type TRow = [bool; 52];
pub type TGrid = [TRow; 52];

// Union to view grid as either rows or flat vector
#[repr(C)]
pub union GridUnion {
    pub as_rows: TGrid,
    pub as_vector: [bool; 52 * 52],  // 2704 elements
}

impl GridUnion {
    pub fn new() -> Self {
        GridUnion {
            as_rows: [[false; 52]; 52]
        }
    }
    
    pub fn as_grid(&self) -> &TGrid {
        unsafe { &self.as_rows }
    }
    
    pub fn as_grid_mut(&mut self) -> &mut TGrid {
        unsafe { &mut self.as_rows }
    }
    
    pub fn as_vector_mut(&mut self) -> &mut [bool; 52 * 52] {
        unsafe { &mut self.as_vector }
    }
}