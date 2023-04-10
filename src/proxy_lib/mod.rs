use std::time::SystemTime;
use std::any::Any;
use std::net::{SocketAddr};
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool}; //, Ordering};
use serde::{Serialize, Deserialize};
// use serde::de::{DeserializeOwned};
// use crate::message_serialisation::{send, ReturnVar, Message, InvokeOp};
// use crate::utils::{fn_type_name};

use umi_macros::*;
// use umi_macros::{IsLocal, ToVariable, ToVariableRef, ToVariableMut, Variable, ID, IsProxyType, SerializeTag};
// use umi_macros_proc::{IsLocal, ToVariable, ToVariableRef, ToVariableMut, IsProxyType, SerializeTag};

pub trait BorrowRemote {
    fn borrow_remote(&self) -> Self;
}

// ToVariableL, ToVariableRefL, ToVariableMutL are for wrapping library types
pub trait ToVariableL {
    fn to_variable(self) -> Variable;
}

pub trait ToVariableRefL {
    fn to_variable(&self) -> Variable;
}

pub trait ToVariableMutL {
    fn to_variable(&mut self) -> Variable;
}

pub trait Retrieve {
    fn retrieve(&self) -> Self;
}

pub trait Proxy {
    fn construct_remote(addr: SocketAddr, id: ID, is_owner: Arc<AtomicBool>) -> Self;
}

pub trait SerializeTagL {
    fn tagged_string(&self) -> (String, bool); // bool - is_local
}

impl SerializeTagL for () {
    fn tagged_string(&self) -> (String, bool) {
        ("Empty".to_string(), true)
    }
}

static mut REFS: Vec<Box<dyn Any>> = Vec::new(); 

impl ToVariableL for SystemTime {
    fn to_variable(self) -> Variable {
        let var = Variable::OwnedLocal(serde_json::to_string(&self).unwrap());
        var
    }
}

impl<T> ToVariableL for Option<T> 
    where
    T: Serialize,
{
    fn to_variable(self) -> Variable {
        let var = Variable::OwnedLocal(serde_json::to_string(&self).unwrap());
        var
    }
}

impl ToVariableL for u32 {
    fn to_variable(self) -> Variable {
        let var = Variable::OwnedLocal(serde_json::to_string(&self).unwrap());
        var
    }
}

impl ToVariableL for String {
    fn to_variable(self) -> Variable {
        let var = Variable::OwnedLocal(serde_json::to_string(&self).unwrap());
        var
    }
}

impl ToVariableL for bool {
    fn to_variable(self) -> Variable {
        let var = Variable::OwnedLocal(serde_json::to_string(&self).unwrap());
        var
    }
}

impl BorrowRemote for String {
    fn borrow_remote(&self) -> Self {
        panic!("This should never be called");
    }
}

impl BorrowRemote for u32 {
    fn borrow_remote(&self) -> Self {
        panic!("This should never be called");
    }
}

impl BorrowRemote for () {
    fn borrow_remote(&self) -> Self {
        panic!("This should never be called");
    }
}

impl BorrowRemote for usize {
    fn borrow_remote(&self) -> Self {
        panic!("This should never be called");
    }
}

impl BorrowRemote for bool {
    fn borrow_remote(&self) -> Self {
        panic!("This should never be called");
    }
}

impl BorrowRemote for SystemTime {
    fn borrow_remote(&self) -> Self {
        panic!("This should never be called");
    }
}

impl<T> BorrowRemote for Option<T> 
    where
    T: Serialize,
{
    fn borrow_remote(&self) -> Self {
        panic!("This should never be called");
    }
}

impl<T> SerializeTagL for Option<T> 
where
T: Serialize,
{
    fn tagged_string(&self) -> (String, bool) {
        let serialised = serde_json::to_string(&self).unwrap();
        (serialised, true)
    }
}

impl SerializeTagL for bool {
    fn tagged_string(&self) -> (String, bool) {
        let serialised = serde_json::to_string(&self).unwrap();
        (serialised, true)
    }
}