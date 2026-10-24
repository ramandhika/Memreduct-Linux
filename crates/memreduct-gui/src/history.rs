use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct HistoryBuffer {
    max_len: usize,
    data: VecDeque<f64>,
}

impl HistoryBuffer {
    pub fn new(max_len: usize) -> Self {
        Self {
            max_len,
            data: VecDeque::with_capacity(max_len),
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.data.len() >= self.max_len {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    pub fn points(&self) -> &VecDeque<f64> {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
