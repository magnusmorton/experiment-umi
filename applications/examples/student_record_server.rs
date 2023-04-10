use std::any::{Any};
use std::sync::{Arc, Mutex};
use umi::endpoint::{UMIEndpoint, ResourceTable};
use umi::registry::{RegistryTable};
use umi::{register};
use applications::umi_demo::students::{Student, StudentRecord};

use umi_macros_proc::{setup_packages, setup_registry, setup_proc_macros};
setup_packages!();
setup_registry!();
setup_proc_macros!();

fn main() {
    let mut table = RegistryTable::new();
    register!(table, // method registry table
        StudentRecordNew, // method registry name
        StudentRecord::new, // method name
        fn() -> StudentRecord, // method signature
        (StudentRecord, ResultOp::Owned)); // method return type and ownership
    register!(table, // method registry table
        StudentRecordAdd, // method registry name
        StudentRecord::add_student, // method name
        fn(&mut StudentRecord, Student),  // method signature
        ((), ResultOp::Owned), // method return type and ownership
        StudentRecord, Student, // argument types
        &mut StudentRecord, String); // argument ownership
    register!(table, // method registry table
        StudentRecordHas, // method registry name
        StudentRecord::has_student, // method name
        fn(&StudentRecord, Student) -> bool, // method signature
        (bool, ResultOp::Owned), // method return type and ownership
        StudentRecord, Student, // argument types
        &StudentRecord, Student); // argument ownership

    let mut server = UMIEndpoint::new("127.0.0.1:3334");
    let vtable = Arc::new(Mutex::new(ResourceTable::new()));
    server.start(table, vtable);
}