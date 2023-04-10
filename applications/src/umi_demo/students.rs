use umi_macros_proc::{proxy_me, umi_init, umi_struct_method, setup_packages, setup_registry, setup_proc_macros};

setup_packages!();
setup_registry!();
setup_proc_macros!();


pub type Student = String;

#[proxy_me]
pub struct StudentRecord {
    students: Vec<Student>
}

impl StudentRecord {
    #[umi_init]
    pub fn new() -> Self {
        StudentRecord {
            students: Vec::new()
        }
    }

    #[umi_struct_method]
    pub fn add_student(&mut self, student: Student) {
        (&mut self.students).push(student);
    }

    #[umi_struct_method(false)]
    pub fn has_student(&self, student: Student) -> bool {
        (&self.students).contains(&student)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn student_record_works() {
        let mut record = StudentRecord::new();
        record.add_student("Jane".to_string());
        assert!(record.has_student("Jane".to_string()));
    }
}