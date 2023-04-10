use serde::{Serialize, Deserialize};
use std::net::{TcpStream, ToSocketAddrs, Shutdown, SocketAddr};
use std::io::{Read, Write, BufReader, BufRead, BufWriter};
use std::any::{TypeId};
use std::fmt::{Debug};
use crate::registry::{RegistryTable};
use crate::utils::{fn_type_name};

use umi_macros::{IsLocal, ToVariable, ToVariableRef, ToVariableMut, Variable, ID};

/* The variable representing a return */
#[derive(Serialize, Deserialize, Debug)]
pub enum ReturnVar {
    Owned(String), // either local or remote
    OwnedInit(SocketAddr, ID, bool), // has to be a proxy, i.e., remote
    RefOwned(SocketAddr, ID), // a reference owning a reference on the remote machine
    RefBorrow(String), // a reference borrowing resource on a remote machine
    MutRefOwned(SocketAddr, ID), // a mutable reference owning a reference on the remote machine
    MutRefBorrow(String), // a mutable reference borrowing resource on a remote machine
}

#[derive(Serialize, Deserialize, Debug)]
pub enum InvokeOp {
    Owned, // result pass by copy / move
    Ref, // result pass by reference
    MutRef, // // result pass by mutable reference
    Init // remote initialisation call
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Invoke(String, Vec<Variable>, InvokeOp),  // (function_name, variables, return_option)
    Return(ReturnVar), // return a variable with one of return variable representations
    Drop(ID) // deallocate remotely owned resource 
}

pub type BadResponseError = String;

pub fn send<A: ToSocketAddrs>(addr: A, msg: Message) -> Result<String, BadResponseError> {
    match TcpStream::connect(addr) {
        Ok(mut stream) => {
            println!("Successfully connected to server");
            let stream_clone = stream.try_clone().unwrap();
            let s_msg = serde_json::to_string(&msg).unwrap();
            let mut writer = BufWriter::new(stream);
            writer.write(s_msg.as_bytes()).unwrap();
            writer.write(b"\n").unwrap();
            writer.flush().unwrap();
            println!("Message Sent");

            let mut data = String::new(); // string buffer
            let mut reader = BufReader::new(stream_clone);
            match reader.read_line(&mut data) {
                Ok(_) => {
                    Ok(data)
                },
                Err(e) => {
                    println!("Failed to receive data: {}", e);
                    Err("Uable to receive data from server".to_string())
                }
            }
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
            Err("Connection Failure".to_string())
        }
    }
}

pub fn response(mut stream: TcpStream, result: Message) {
    let stream_clone = stream.try_clone().unwrap();
    let mut writer = BufWriter::new(stream_clone);
    let response_data: String = serde_json::to_string(&result).unwrap();
    writer.write(response_data.as_bytes()).unwrap();
    writer.write(b"\n").unwrap();
    writer.flush().unwrap();
}