use std::time::{SystemTime, Duration};
use std::{thread};
use umi::{remote};
use applications::reminder::ready_reminder_server_multi::{ReadyReminderServer};
use umi_macros_proc::{setup_packages};
setup_packages!();

fn main() {
    let mut r = remote!("127.0.0.1:3335", ReadyReminderServer::new, ReadyReminderServer);
    //let mut r = ReadyReminderServer::new();
    r.submit_event("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0));
    r.submit_event("Hello World!".to_string(), SystemTime::now() + Duration::new(1, 0));
    println!("The first event is: {:?}", r.extract_event());
    thread::sleep(Duration::new(4, 0));
    println!("The first event is: {:?}", r.extract_event());
    println!("The first event is: {:?}", r.extract_event());
}