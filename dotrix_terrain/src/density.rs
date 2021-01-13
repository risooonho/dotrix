use std::collections::HashMap;

use rayon::prelude::*;

struct Value {
    y: usize,
    value: f32,
}

pub struct Density {
    size_x: usize,
    size_y: usize,
    size_z: usize,
    values: Option<Vec<Vec<Vec<f32>>>>,
}

impl Density {
    pub fn new(size_x: usize, size_y: usize, size_z: usize) -> Self {
        Self {
            size_x,
            size_y,
            size_z,
            values: None,
        }
    }

    pub fn set<F>(&mut self, density: F)
    where F: Fn(usize, usize, usize) -> f32 + Send + Sync
    {
        let values = (0..self.size_x).into_par_iter().map(|x| {
            (0..self.size_y).into_par_iter().map(|y| {
                (0..self.size_z).into_par_iter().map(|z| {
                    density(x, y, z)
                }).collect::<Vec<_>>()
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>();
        self.values = Some(values);
    }

    pub fn value(&self, x: usize, y: usize, z: usize) -> f32 {
        if let Some(values) = self.values.as_ref() {
            values[x][y][z]
        } else {
            1.0
        }
    }
}
