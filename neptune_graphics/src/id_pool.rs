pub struct IdPool {
    freed_ids: Vec<u32>,
    next_id: u32,
    max_id: Option<u32>,
}

impl IdPool {
    pub fn new(first_id: u32) -> Self {
        Self {
            freed_ids: Vec::new(),
            next_id: first_id,
            max_id: None,
        }
    }

    pub fn new_with_max(first_id: u32, max_id: u32) -> Self {
        Self {
            freed_ids: Vec::new(),
            next_id: first_id,
            max_id: Some(max_id),
        }
    }

    pub fn get(&mut self) -> u32 {
        if let Some(id) = self.freed_ids.pop() {
            id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            if let Some(max_id) = self.max_id {
                if id >= max_id {
                    panic!("Failed to allocate new id, max value reached");
                }
            }
            id
        }
    }

    pub fn free(&mut self, id: u32) {
        self.freed_ids.push(id);
    }
}
