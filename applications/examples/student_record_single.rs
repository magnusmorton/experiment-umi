
use applications::umi_demo::students_single::{StudentRecord};

fn main() {
    let mut record = StudentRecord::new();
    record.add_student("Jane Doe".to_string());
    println!("Has student John Doe? : {:?}", record.has_student("John Doe".to_string()));
    println!("Has student Jane Doe? : {:?}", record.has_student("Jane Doe".to_string()));
}