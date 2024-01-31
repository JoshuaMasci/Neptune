pub struct IdPool {
    id_range: std::ops::Range<usize>,
    freed_ids: Vec<usize>,
}

impl IdPool {
    pub fn new(id_range: std::ops::Range<usize>) -> Self {
        Self {
            id_range,
            freed_ids: Vec::new(),
        }
    }

    pub fn get(&mut self) -> Option<usize> {
        self.freed_ids.pop().or_else(|| self.id_range.next())
    }

    pub fn free(&mut self, id: usize) {
        self.freed_ids.push(id);
    }
}
