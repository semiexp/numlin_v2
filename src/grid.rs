use std::ops::{Index, IndexMut};

pub struct Grid<T> {
    height: usize,
    width: usize,
    data: Vec<T>,
}

impl<T> Grid<T> {
    pub fn new(height: usize, width: usize, default_value: T) -> Self
    where
        T: Clone,
    {
        Self {
            height,
            width,
            data: vec![default_value; height * width],
        }
    }

    pub fn flat_index(&self, p: (usize, usize)) -> usize {
        let (row, col) = p;
        assert!(row < self.height && col < self.width, "Index out of bounds");
        row * self.width + col
    }
}

impl<T> Index<(usize, usize)> for Grid<T> {
    type Output = T;

    fn index(&self, p: (usize, usize)) -> &Self::Output {
        &self.data[self.flat_index(p)]
    }
}

impl<T> IndexMut<(usize, usize)> for Grid<T> {
    fn index_mut(&mut self, p: (usize, usize)) -> &mut Self::Output {
        let i = self.flat_index(p);
        &mut self.data[i]
    }
}

impl<T> Index<i32> for Grid<T> {
    type Output = T;

    fn index(&self, idx: i32) -> &Self::Output {
        assert!(
            idx >= 0 && (idx as usize) < self.data.len(),
            "Index out of bounds"
        );
        let idx = idx as usize;
        &self.data[idx]
    }
}

impl<T> IndexMut<i32> for Grid<T> {
    fn index_mut(&mut self, idx: i32) -> &mut Self::Output {
        assert!(
            idx >= 0 && (idx as usize) < self.data.len(),
            "Index out of bounds"
        );
        let idx = idx as usize;
        &mut self.data[idx]
    }
}
