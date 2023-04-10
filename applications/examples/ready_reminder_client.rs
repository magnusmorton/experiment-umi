use std::time::{SystemTime, Duration};
use std::{thread};
use applications::reminder::ready_reminder_server::{ReadyReminderServer};

fn main() {
    let mut r = ReadyReminderServer::new();
    r.submit_event("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0));
    r.submit_event("Hello World!".to_string(), SystemTime::now() + Duration::new(1, 0));
    println!("The first event is: {:?}", r.extract_event());
    thread::sleep(Duration::new(4, 0));
    println!("The first event is: {:?}", r.extract_event());
    println!("The first event is: {:?}", r.extract_event());
}