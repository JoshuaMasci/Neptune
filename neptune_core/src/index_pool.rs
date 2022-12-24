pub struct IndexPool<T: Copy + PartialOrd> {
    freed_indexes: Vec<T>,
    next_index: T,
    index_range: std::ops::Range<T>,
}

impl<T: Copy + PartialOrd> IndexPool<T> {
    pub fn new(range: std::ops::Range<T>) -> Self {
        Self {
            freed_indexes: Vec::new(),
            next_index: range.start,
            index_range: range,
        }
    }

    pub fn get(&mut self) -> Option<T> {
        if let Some(index) = self.freed_indexes.pop() {
            Some(index)
        } else {
            let index = self.next_index;
            if self.index_range.contains(&index) {
                Some(index)
            } else {
                None
            }
        }
    }

    pub fn free(&mut self, index: T) {
        if self.index_range.contains(&index) {
            self.freed_indexes.push(index);
        }
    }
}
