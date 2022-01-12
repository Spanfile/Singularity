use indexmap::IndexMap;
use nanoid::nanoid;
use std::time::Instant;

pub struct TimedCollection<T> {
    values: IndexMap<String, Entry<T>>,
    max_entries: usize,
    max_entry_lifetime: u64,
}

#[derive(Debug)]
struct Entry<T> {
    value: T,
    time: Instant,
}

impl<T> TimedCollection<T> {
    pub fn new(max_entries: usize, max_entry_lifetime: u64) -> Self {
        Self {
            values: IndexMap::new(),
            max_entries,
            max_entry_lifetime,
        }
    }

    pub fn add(&mut self, value: T) -> String {
        let time = Instant::now();

        // ensure a duplicate ID won't be generated
        let id = loop {
            let id = nanoid!();
            if !self.values.contains_key(&id) {
                break id;
            }
        };

        self.values.insert(id.clone(), Entry { value, time });
        self.cleanup();
        id
    }

    pub fn remove(&mut self, id: &str) {
        self.values.remove(id);
        self.cleanup();
    }

    pub fn get_ref(&self, id: &str) -> Option<&T> {
        self.values.get(id).map(|entry| &entry.value)
    }

    pub fn get(&mut self, id: &str) -> Option<T> {
        let value = self.values.remove(id).map(|import| import.value)?;
        self.cleanup();

        Some(value)
    }

    pub fn cleanup(&mut self) {
        loop {
            // keep removing imports until there's as much as concurrently allowed
            if self.values.len() > self.max_entries {
                self.values.pop();
                continue;
            }

            // pop imports until such is hit that is younger than the maximum allowed lifetime, therefore any imports
            // after it are also younger, or that there aren't any imports left
            if let Some((_, last)) = self.values.last() {
                if last.time.elapsed().as_secs() >= self.max_entry_lifetime {
                    self.values.pop();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
}
