use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use std::net::{SocketAddr};

// The ID in endpoint will be replaced by this
pub type ID = (SystemTime, usize);

// The Variable in message_serialisation will be replaced by this
#[derive(Serialize, Deserialize, Debug)]
pub enum Variable {
    OwnedLocal(String), // (serialised_local)
    OwnedRemote(String, SocketAddr, ID), // (serialised_remote, address, id)
    RefRemote(String, SocketAddr, ID), // (serialised_remote, address, id)
    MutRefRemote(String, SocketAddr, ID) // (serialised_remote, address, id)
}

impl Variable {
    pub fn is_ref(&self) -> bool {
        match self {
            Variable::OwnedLocal(..) | Variable::OwnedRemote(..) => { false },
            _ => true
        }
    }
}

pub trait IsLocal {
    fn is_local(&self) -> bool;
}

pub trait ToVariable {
    fn to_variable(self) -> Variable;
}

pub trait ToVariableRef {
    fn to_variable(&self) -> Variable;
}

pub trait ToVariableMut {
    fn to_variable(&mut self) -> Variable;
}

pub trait DropMarker {
}

pub trait IsProxyType {
    fn is_proxy_type(&self) -> bool;
}

pub trait SerializeTag {
    fn tagged_string(&self) -> (String, bool); // bool - is_local
}

pub trait BorrowRemoteMarker {
}