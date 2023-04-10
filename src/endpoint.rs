use std::thread;
use std::any::Any;
use std::net::{TcpListener, TcpStream, Shutdown, ToSocketAddrs};
use std::collections::{HashMap};
use std::time::SystemTime;
use std::io::{Read, BufReader, BufRead};
use std::sync::{Arc, RwLock, Mutex, mpsc};
use std::cell::{RefCell, Ref, RefMut};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use serde::{Serialize, Deserialize};
use crate::message_serialisation::{Message, ReturnVar, send, response, InvokeOp};
use crate::registry::{RegistryTable, Argument};

use umi_macros::{Variable, ID};

//pub type ID = (SystemTime, usize);

pub type ResourceTable = HashMap<ID, RefCell<(Box<dyn Any + Send + Sync>, bool)>>;

pub struct IDGen {
    id: usize
}

impl IDGen {
    pub fn new() -> IDGen {
        IDGen { id: 0 }
    }

    pub fn next(&mut self) -> usize {
        let curr = self.id;
        self.id += 1;
        curr
    }
}

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

pub struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

pub struct UMIEndpoint {
    listener: TcpListener,
}

impl UMIEndpoint {
    pub fn new<A: ToSocketAddrs>(addr: A) -> UMIEndpoint {
        UMIEndpoint {
            listener: TcpListener::bind(addr).unwrap(),
        }
    }

    pub fn start(&mut self, registry_table: RegistryTable, vtable: Arc<Mutex<ResourceTable>>) {
        let id_gen = Arc::new(Mutex::new(IDGen::new()));
        let rtable = Arc::new(Mutex::new(registry_table.clone()));
        // bool - is an entry for resouce or reference
        //let vtable: Arc<Mutex<ResourceTable>> = Arc::new(Mutex::new(resource_table));
        let local_address = self.listener.local_addr().unwrap();

        let pool = ThreadPool::new(5);
        for stream in self.listener.incoming() {
            let id_gen = Arc::clone(&id_gen);
            let rtable = Arc::clone(&rtable);
            let vtable = Arc::clone(&vtable);
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    pool.execute(move|| {
                        // connection succeeded
                        // deserialisation, invocation, serialise result, send back
                        let mut reader = BufReader::new(&stream);
                        let mut data = String::new();
                        let mut m_id_gen = id_gen.lock().expect("Something went wrong with the id generator");
                        let mut mvtable = vtable.lock().unwrap();
                        let lrtable = rtable.lock().unwrap();
                        match reader.read_line(&mut data) {
                            Ok(size) => {
                                let deserialised: Message = serde_json::from_str(&data).unwrap();
                                //println!("{:?}", deserialised);
                                match deserialised {
                                    Message::Return(_) => {
                                        println!("Not a valid invocation request");
                                    },
                                    Message::Drop(id) => {
                                        println!("Dropping resource with id: {:?}", &id);
                                        mvtable.remove(&id);
                                        println!("The size of the vtable is: {}", mvtable.len());
                                    },
                                    Message::Invoke(fname, variables, invoke_op) => {
                                        let mut arguments: Vec<Argument> = Vec::new();
                                        for v in &variables {
                                            match v {
                                                Variable::OwnedLocal(s) => {
                                                    arguments.push(Argument::Serialised(s.clone()));
                                                },
                                                Variable::OwnedRemote(serialise_remote, addr, id) => {
                                                    if addr == &local_address { // the resource of a proxy indeed lives on this machine
                                                        let (owned, is_ref) = mvtable.remove(id).unwrap().into_inner(); // is_ref here should never br true
                                                        let arg_ref = Argument::Owned(owned);
                                                        arguments.push(arg_ref);
                                                    } else { // the resource of a remote proxy does no live on this machine -- just push the remote reference in, for later invocation
                                                        arguments.push(Argument::Serialised(serialise_remote.to_string()));
                                                    }
                                                },
                                                Variable::RefRemote(serialise_remote, addr, id) => {
                                                    if addr == &local_address { // the resource of a remote reference indeed lives on this machine
                                                        let borrow = mvtable.get(id).unwrap().borrow();
                                                        let ptr: *const (Box<dyn Any + Send + Sync>, bool) = &*borrow;
                                                        unsafe {
                                                            let back: &(Box<dyn Any + Send + Sync>, bool) = ptr.as_ref().unwrap();
                                                            let arg_ref = Argument::Ref(&back.0, back.1);
                                                            arguments.push(arg_ref);
                                                        }
                                                    } else { // the resource of a remote reference does not live on this machine -- just push the remote reference in, for later invocation
                                                        arguments.push(Argument::RemoteRef(serialise_remote.to_string()));
                                                    }
                                                },
                                                Variable::MutRefRemote(serialise_remote, addr, id) => {
                                                    if addr == &local_address { // the resource of a remote reference indeed lives on this machine
                                                        let mut borrow_mut = mvtable.get(id).unwrap().borrow_mut();
                                                        let ptr: *mut (Box<dyn Any + Send + Sync>, bool) = &mut *borrow_mut;
                                                        unsafe {
                                                            let back: &mut (Box<dyn Any + Send + Sync>, bool) = ptr.as_mut().unwrap();
                                                            let arg_ref = Argument::MutRef(&mut back.0, back.1);
                                                            arguments.push(arg_ref);
                                                        }
                                                    } else { // the resource of a remote reference does not live on this machine -- just push the remote reference in, for later invocation
                                                        arguments.push(Argument::RemoteMutRef(serialise_remote.clone()));
                                                    }
                                                }
                                            }
                                        }
                                        let f: &str = &*fname; 
                                        match lrtable.get(f) {
                                            Some(f) => {
                                                let ((res, is_local), b) = f.call(arguments);
                                                let res_message: Message;
                                                match invoke_op {
                                                    InvokeOp::Owned => {
                                                        // The result is pass back with the result or thr proxy:
                                                        // - A::Local
                                                        // - A::Remote
                                                        res_message = Message::Return(ReturnVar::Owned(res));
                                                    },
                                                    InvokeOp::Init => {
                                                        // This is the initalisation call, requiring a proxy to be sent back to the caller
                                                        // while the resouce owned by the proxy is stored in the reserver:
                                                        // - A::Remote
                                                        let id = (SystemTime::now(), m_id_gen.next());
                                                        mvtable.insert(id.clone(), RefCell::new((b, false))); // b is the resource
                                                        res_message = Message::Return(ReturnVar::OwnedInit(local_address, id, true));
                                                    },
                                                    InvokeOp::Ref => { // borrow
                                                        if is_local { // the local reference is boxed and inserted to the table, the proxy points to the local reference
                                                            let id = (SystemTime::now(), m_id_gen.next());
                                                            mvtable.insert(id.clone(), RefCell::new((b, true))); // b is a reference
                                                            res_message = Message::Return(ReturnVar::RefOwned(local_address, id));
                                                        } else {
                                                            res_message = Message::Return(ReturnVar::RefBorrow(res));
                                                        }
                                                    },
                                                    InvokeOp::MutRef => { // mutable borrow
                                                        if is_local { // the local reference is boxed and inserted to the table, the proxy points to the local reference
                                                            let id = (SystemTime::now(), m_id_gen.next());
                                                            mvtable.insert(id.clone(), RefCell::new((b, true))); // b is a reference
                                                            res_message = Message::Return(ReturnVar::MutRefOwned(local_address, id));
                                                        } else {
                                                            res_message = Message::Return(ReturnVar::MutRefBorrow(res));
                                                        }
                                                    }
                                                }
                                                response(stream, res_message);
                                            },
                                            None => {
                                                println!("{}", "no such function found");
                                            }
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                println!("An error: {} occurred, terminating connection", e);
                                stream.shutdown(Shutdown::Both).unwrap();
                            }
                        }
                    });
                }
                Err(e) => {
                    println!("Error: {}", e);
                    /* connection failed */
                }
            }
        }
    }

    pub fn close(&self) {
        drop(&self.listener);
    }
}

#[macro_export]
macro_rules! remote {
    ($addr:expr, $fn_name:path, $return_ty:ty $(, $x:expr )*) => { 
        {
            let mut vec = Vec::new();
            $(
                vec.push($x.to_variable());
            )*
            let msg = Message::Invoke(fn_type_name(&$fn_name).to_string(), vec, InvokeOp::Init);
            let res = send($addr, msg).unwrap();
            let res_msg : Message = serde_json::from_str(&res).unwrap();
            let result: $return_ty;
            match res_msg {
                Message::Return(var) => {
                    match var {
                        ReturnVar::Owned(s) => {
                            result = serde_json::from_str(&s).unwrap();
                        },
                        ReturnVar::OwnedInit(addr, id, is_owner) => {
                            result = <$return_ty>::Remote(addr, id, Arc::new(AtomicBool::new(is_owner)));
                        },
                        _ => {panic!("Invalid return value")}     
                    }
                },
                _ => {panic!("I am expecting a return message.")}
            }
            result
        }
    };
}


#[cfg(test)]
mod test {
    use crate::endpoint::{UMIEndpoint};
    use std::any::Any;

}