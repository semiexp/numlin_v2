#[derive(Debug, Clone)]
pub struct Problem {
    height: usize,
    width: usize,
    data: Vec<Option<i32>>,
}

impl Problem {
    pub fn new(height: usize, width: usize) -> Self {
        Self {
            height,
            width,
            data: vec![None; height * width],
        }
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn set(&mut self, row: usize, col: usize, value: Option<i32>) {
        assert!(row < self.height && col < self.width, "Index out of bounds");
        self.data[row * self.width + col] = value;
    }

    pub fn get(&self, row: usize, col: usize) -> Option<i32> {
        assert!(row < self.height && col < self.width, "Index out of bounds");
        self.data[row * self.width + col]
    }
}
