pub type Student = String;

pub struct StudentRecord {
    students: Vec<Student>
}

impl StudentRecord {
    pub fn new() -> Self {
        StudentRecord {
            students: Vec::new()
        }
    }
    pub fn add_student(&mut self, student: Student) {
        (&mut self.students).push(student);
    }

    pub fn has_student(&self, student: Student) -> bool {
        (&self.students).contains(&student)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn student_single_record_works() {
        let mut record = StudentRecord::new();
        record.add_student("Jane".to_string());
        assert!(record.has_student("Jane".to_string()));
    }
}