use std::any::{type_name};

pub fn fn_type_name<T: 'static> (_: &T) -> &'static str {
    type_name::<T>()
}