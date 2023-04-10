use std::sync::atomic::{AtomicBool};
use std::sync::{Arc};
use umi::{remote};
use applications::umi_demo::students::{StudentRecord};
use umi_macros_proc::{setup_packages};
setup_packages!();

fn main() {
    let mut record = remote!("127.0.0.1:3334", StudentRecord::new, StudentRecord);
    record.add_student("Jane Doe".to_string());
    println!("Has student John Doe? : {:?}", record.has_student("John Doe".to_string()));
    println!("Has student Jane Doe? : {:?}", record.has_student("Jane Doe".to_string()));
}