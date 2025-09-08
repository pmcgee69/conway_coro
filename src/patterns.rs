use crate::TGrid;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Pattern {
    pub name: &'static str,
    pub cells: &'static [(usize, usize)],
}

pub const PATTERNS: &[Pattern] = &[
    Pattern {
        name: "Glider",
        cells: &[(6, 7), (7, 8), (8, 6), (8, 7), (8, 8)],
    },
    Pattern {
        name: "Blinker",
        cells: &[(25, 24), (25, 25), (25, 26)],
    },
    Pattern {
        name: "Toad",
        cells: &[(24, 25), (24, 26), (24, 27), (25, 24), (25, 25), (25, 26)],
    },
    Pattern {
        name: "Beacon",
        cells: &[(10, 10), (10, 11), (11, 10), (11, 11), (12, 12), (12, 13), (13, 12), (13, 13)],
    },
    Pattern {
        name: "Pulsar",
        cells: &[
            // Top section
            (20, 24), (20, 25), (20, 26), (20, 30), (20, 31), (20, 32),
            (22, 22), (22, 27), (22, 29), (22, 34),
            (23, 22), (23, 27), (23, 29), (23, 34),
            (24, 22), (24, 27), (24, 29), (24, 34),
            (25, 24), (25, 25), (25, 26), (25, 30), (25, 31), (25, 32),
            // Bottom section (mirrored)
            (27, 24), (27, 25), (27, 26), (27, 30), (27, 31), (27, 32),
            (28, 22), (28, 27), (28, 29), (28, 34),
            (29, 22), (29, 27), (29, 29), (29, 34),
            (30, 22), (30, 27), (30, 29), (30, 34),
            (32, 24), (32, 25), (32, 26), (32, 30), (32, 31), (32, 32),
        ],
    },
    Pattern {
        name: "R-pentomino",
        cells: &[(25, 25), (25, 26), (24, 26), (26, 25), (26, 24)],
    },
    Pattern {
        name: "Gosper Glider Gun",
        cells: &[
            (5, 1), (5, 2), (6, 1), (6, 2),
            (5, 11), (6, 11), (7, 11), (4, 12), (8, 12), (3, 13), (9, 13),
            (3, 14), (9, 14), (6, 15), (4, 16), (8, 16), (5, 17), (6, 17),
            (7, 17), (6, 18), (3, 21), (4, 21), (5, 21), (3, 22), (4, 22),
            (5, 22), (2, 23), (6, 23), (1, 25), (2, 25), (6, 25), (7, 25),
            (3, 35), (4, 35), (3, 36), (4, 36),
        ],
    },
];

pub fn apply_pattern(grid: &mut TGrid, pattern: &Pattern) {
    // Clear grid first
    *grid = [[false; 52]; 52];
    
    // Apply pattern
    for &(row, col) in pattern.cells {
        if row >= 1 && row <= 50 && col >= 1 && col <= 50 {
            grid[row][col] = true;
        }
    }
}

pub fn apply_random_pattern(grid: &mut TGrid, seed_value: u32) {
    // Clear everything first
    *grid = [[false; 52]; 52];
    
    // Simple pseudo-random generator
    let mut hasher = DefaultHasher::new();
    seed_value.hash(&mut hasher);
    let mut seed = hasher.finish();
    
    // Only fill the active area (1-50)
    for row in 1..51 {
        for col in 1..51 {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            grid[row][col] = (seed % 3) == 0; // ~33% chance of being alive
        }
    }
}