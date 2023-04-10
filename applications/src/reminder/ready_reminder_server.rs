use std::time::SystemTime;
use std::collections::BinaryHeap;
use std::sync::atomic::{Ordering};
use serde::{Serialize, Deserialize};

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    content: String,
    time: SystemTime
}

impl Entry {
    pub fn new(content: String, ready_at: SystemTime) -> Entry {
        Entry {
            content: content,
            time: ready_at
        }
    }

    pub fn get_content(&self) -> &String {
        &self.content
    }

    pub fn get_time(&self) -> &SystemTime {
        &self.time
    }
}

// partial order by time
impl PartialOrd for Entry {
    /* this is manually changed for builing a min-heap with std BinaryHeap */
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.time.partial_cmp(&self.time)
    }

    fn lt(&self, other: &Self) -> bool {
        self.time > other.time
    }

    fn le(&self, other: &Self) -> bool {
        self.time >= other.time
    }

    fn gt(&self, other: &Self) -> bool {
        self.time < other.time
    }

    fn ge(&self, other: &Self) -> bool {
        self.time <= other.time
    }
}

// total order by time
impl Ord for Entry {
    /* this is manually changed for builing a min-heap with std BinaryHeap */
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub struct ReadyReminderServer {
    entries: BinaryHeap<Entry>
}

impl ReadyReminderServer {
    pub fn new() -> ReadyReminderServer {
        ReadyReminderServer {
            entries: BinaryHeap::new()
        }
    }

    pub fn submit_event(&mut self, content: String, ready_at: SystemTime) {
        let entry = Entry::new(content, ready_at);
        (&mut self.entries).push(entry);
    }

    pub fn extract_event(&mut self) -> Option<Entry> {
        let first = (&self.entries).peek();
        match first {
            Some(entry) => {
                if entry.get_time() <= &SystemTime::now() {
                    let e = (&mut self.entries).pop();
                    return e;
                } else {
                    return None;
                }
            },
            None => {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, Duration};

    #[test]
    fn gt_works() {
        let e1 = Entry::new("Hello World!".to_string(), SystemTime::now() + Duration::new(1, 0));
        let e2 = Entry::new("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0));
        assert!(e1 > e2);
    }

    #[test]
    fn cmp_works() {
        let e1 = Entry::new("Hello World!".to_string(), SystemTime::now() + Duration::new(1, 0));
        let e2 = Entry::new("Goodbye World!".to_string(), SystemTime::now() + Duration::new(2, 0));
        assert_eq!(e1.cmp(&e2), std::cmp::Ordering::Greater);
    }

    #[test]
    fn heap_works() {
        let mut h = BinaryHeap::new();
        let e1 = Entry::new("Hello World!".to_string(), SystemTime::now() + Duration::new(1, 0));
        let e2 = Entry::new("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0));
        h.push(e2);
        h.push(e1);
        assert!(h.peek().unwrap().get_content() == "Hello World!");
    }

    #[test]
    fn can_submit() {
        let mut server = ReadyReminderServer::new();
        server.submit_event("Hello World!".to_string(), SystemTime::now() + Duration::new(0, 0));
        match server.extract_event() {
            Some(e) => {
                assert!(e.get_content() == "Hello World!");
            },
            None => {
                panic!("Should have returned an event");
            }
        }
    }
}
