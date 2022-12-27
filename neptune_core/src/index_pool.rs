pub struct IndexPool<T: Copy + PartialOrd> {
    freed_indexes: Vec<T>,
    next_index: T,
    index_range: Option<std::ops::Range<T>>,
}

impl<T: Copy + PartialOrd> IndexPool<T> {
    pub fn new(starting_value: T) -> Self {
        Self {
            freed_indexes: Vec::new(),
            next_index: starting_value,
            index_range: None,
        }
    }

    pub fn new_range(range: std::ops::Range<T>) -> Self {
        Self {
            freed_indexes: Vec::new(),
            next_index: range.start,
            index_range: Some(range),
        }
    }

    pub fn get(&mut self) -> Option<T> {
        if let Some(index) = self.freed_indexes.pop() {
            Some(index)
        } else {
            let index = self.next_index;
            if let Some(index_range) = &self.index_range {
                if index_range.contains(&index) {
                    Some(index)
                } else {
                    None
                }
            } else {
                Some(index)
            }
        }
    }

    pub fn free(&mut self, index: T) {
        if let Some(index_range) = &self.index_range {
            if index_range.contains(&index) {
                self.freed_indexes.push(index);
            }
        } else {
            self.freed_indexes.push(index);
        }
    }
}
