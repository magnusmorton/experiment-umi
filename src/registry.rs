use std::any::{TypeId, Any, type_name};
use std::collections::{HashMap};
use serde::{Serialize, Deserialize};
use serde::de::{DeserializeOwned};
use std::fmt::Debug;
use std::net::{SocketAddr};
use crate::proxy_lib::{BorrowRemote};
use crate::utils::{fn_type_name};

use umi_macros::{SerializeTag};
use umi_macros_proc::{SerializeTag};

pub type RegistryTable = HashMap<&'static str, Box<dyn GenCall>>;

/* For storing proxies which are used as a reference or mutable reference */
static mut REFS: Vec<Box<dyn Any>> = Vec::new(); 

pub struct MutPtr<T> (pub *mut T);
unsafe impl<T: Send> Send for MutPtr<T> { }
unsafe impl<T: Sync> Sync for MutPtr<T> { }

pub struct ConstPtr<T> (pub *const T);
unsafe impl<T: Send> Send for ConstPtr<T> { }
unsafe impl<T: Sync> Sync for ConstPtr<T> { }

/* An argument that is going to passed into the call() function */
pub enum Argument<'a> {
    Serialised(String), // the argument is either a serialised copy or proxy
    Owned(Box<dyn Any + Send + Sync>), // the argument is owned, removed from the vtable
    Ref(&'a Box<dyn Any + Send + Sync>, bool), // the argument is borrowed, retrieved from the vtable
    MutRef(&'a mut Box<dyn Any + Send + Sync>, bool), // the argument is a mutable borrow, retrived from the vtable
    RemoteRef(String), // remote relative to the reciever, String is a serialised proxy
    RemoteMutRef(String) // remote relative to the reciever, String is a serialised proxy
}

// The wrapper, in order to allow the call() function to call on these argument
// For call()'s internal downcasting or deserialisation of these arguments
pub enum WrapArg<'a, T> {
    Owned(T),
    Ref(&'a T),
    MutRef(&'a mut T)
}

impl<'a> Argument<'a> {
    pub fn get_arg<T: 'static + DeserializeOwned + Clone + BorrowRemote>(&'a mut self) -> WrapArg<'a, T> {
        match self {
            Argument::Serialised(s) => {
                let arg: T = serde_json::from_str(&s).unwrap();
                return WrapArg::Owned(arg);
            },
            Argument::Owned(b) => {
                let arg = b.downcast_ref::<T>().unwrap().to_owned();
                return WrapArg::Owned(arg);
            },
            Argument::Ref(b, is_ref) => {
                if *is_ref {
                    let result = &b.downcast_ref::<ConstPtr<T>>();
                    match result {
                        Some(dptr) => {
                            unsafe {
                                let back: &T = dptr.0.as_ref().unwrap();
                                return WrapArg::Ref(back);
                            }
                        },
                        None => { // mut ref -> ref
                            let dptr: &*mut T = &b.downcast_ref::<MutPtr<T>>().unwrap().0;
                            unsafe {
                                let back: &T = dptr.as_ref().unwrap();
                                return WrapArg::Ref(back);
                            }
                        }
                    }
                } else {
                    let arg = b.downcast_ref::<T>().unwrap();
                    return WrapArg::Ref(arg);
                }
            },
            Argument::MutRef(ref mut b, is_ref) => {
                if *is_ref {
                    let dptr: &*mut T = &b.downcast_mut::<MutPtr<T>>().unwrap().0;
                    unsafe {
                        let back: &mut T = dptr.as_mut().unwrap();
                        return WrapArg::MutRef(back);
                    }
                } else {
                    let arg = b.downcast_mut::<T>().unwrap();
                    return WrapArg::MutRef(arg);
                }
            },
            Argument::RemoteRef(s) => {
                let deserialised: T = serde_json::from_str(&s).unwrap();
                let borrow: T = deserialised.borrow_remote();
                let remote: Box<dyn Any> = Box::new(borrow);
                unsafe {
                    REFS.push(remote); // hold the value in global varible for a longer lifetime
                    WrapArg::Ref(REFS.last().unwrap().downcast_ref::<T>().unwrap())
                }
            },
            Argument::RemoteMutRef(s) => {
                let deserialised: T = serde_json::from_str(&s).unwrap();
                let borrow: T = deserialised.borrow_remote();
                let remote: Box<dyn Any> = Box::new(borrow);
                unsafe {
                    REFS.push(remote); // hold the value in global varible for a longer lifetime
                    WrapArg::MutRef(REFS.last_mut().unwrap().downcast_mut::<T>().unwrap())
                }
            }
        }
    }
}

impl <'a, T> Extract<T> for WrapArg<'a, T> {
    fn extract(self) -> T {
        match self {
            WrapArg::Owned(b) => {
                b
            },
            _ => panic!("wrong cases for owned")
        }
    }
}

impl <'a, T> Extract<&'a T> for WrapArg<'a, T> {
    fn extract(self) -> &'a T {
        match self {
            WrapArg::Ref(b) => {
                b
            },
            _ => panic!("wrong cases for ref")
        }
    }
}

impl <'a, T> Extract<&'a mut T> for WrapArg<'a, T> {
    fn extract(self) -> &'a mut T {
        match self {
            WrapArg::MutRef(b) => {
                b
            },
            _ => panic!("wrong cases for mut ref")
        }
    }
}

pub trait GenCall: Send + Sync + GenCallClone {
    fn call(&self, a: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>);
}

pub trait GenCallClone {
    fn clone_box(&self) -> Box<dyn GenCall>;
}

impl<T> GenCallClone for T
where
    T: 'static + GenCall + Clone,
{
    fn clone_box(&self) -> Box<dyn GenCall> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn GenCall> {
    fn clone(&self) -> Box<dyn GenCall> {
        self.clone_box()
    }
}

pub trait Extract<T> {
    fn extract(self) -> T;
}

// pub struct Wrap<'a, T>(pub &'a mut Option<T>);

// impl <'a, T> Extract<T> for Wrap<'a, T> {
//     fn extract(self) -> T {
//         self.0.take().unwrap()
//     }
// }
// impl <'a, T> Extract<&'a T> for Wrap<'a, T> {
//     fn extract(self) -> &'a T {
//         self.0.as_ref().unwrap()
//     }
// }
// impl <'a, T> Extract<&'a mut T> for Wrap<'a, T> {
//     fn extract(self) -> &'a mut T {
//         self.0.as_mut().unwrap()
//     }
// }

pub enum ResultOp {
    Ref,
    MutRef,
    Owned
}

#[macro_export]
macro_rules! register {
    // one arg - return ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty_plain:ty, $res_ty:ty, ResultOp::Ref), $args_ty_plain:ty, $args_ty:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>) {
                let mut arg = args.pop().unwrap();
                let w = arg.get_arg::<$args_ty_plain>();
                let result: $res_ty = (self.ptr)(w.extract());
                let serialised = result.tagged_string();
                let ptr: *const $res_ty_plain = result;
                let boxed: Box<dyn Any + Send + Sync> = Box::new(ConstPtr(ptr));
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // one arg - return mut ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty_plain:ty, $res_ty:ty, ResultOp::MutRef), $args_ty_plain:ty, $args_ty:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>) {
                let mut arg = args.pop().unwrap();
                let w = arg.get_arg::<$args_ty_plain>();
                let result = (self.ptr)(w.extract());
                let serialised = result.tagged_string();
                let ptr: *mut $res_ty_plain = result;
                let boxed: Box<dyn Any + Send + Sync> = Box::new(MutPtr(ptr));
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // one arg - return owned
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty:ty, ResultOp::Owned), $args_ty_plain:ty, $args_ty:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>) {
                let mut arg = args.pop().unwrap();
                let w = arg.get_arg::<$args_ty_plain>();
                let result = (self.ptr)(w.extract());
                let serialised = result.tagged_string();
                let boxed: Box<dyn Any + Send + Sync> = Box::new(result);
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // two args - return ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty_plain:ty, $res_ty:ty, ResultOp::Ref), $args_ty_plain1:ty, $args_ty_plain2:ty, $args_ty1:ty, $args_ty2:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty1, $args_ty2) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>){
                let mut arg2 = args.pop().unwrap();
                let mut arg1 = args.pop().unwrap();
                let w1 = arg1.get_arg::<$args_ty_plain1>();
                let w2 = arg2.get_arg::<$args_ty_plain2>();
                let result = (self.ptr)(w1.extract(), w2.extract());
                let serialised = result.tagged_string();
                let ptr: *const $res_ty_plain = result;
                let boxed: Box<dyn Any + Send + Sync> = Box::new(ConstPtr(ptr));
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // two args - return mut ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty_plain:ty, $res_ty:ty, ResultOp::MutRef), $args_ty_plain1:ty, $args_ty_plain2:ty, $args_ty1:ty, $args_ty2:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty1, $args_ty2) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>){
                let mut arg2 = args.pop().unwrap();
                let mut arg1 = args.pop().unwrap();
                let w1 = arg1.get_arg::<$args_ty_plain1>();
                let w2 = arg2.get_arg::<$args_ty_plain2>();
                let result = (self.ptr)(w1.extract(), w2.extract());
                let serialised = result.tagged_string();
                let ptr: *mut $res_ty_plain = result;
                let boxed: Box<dyn Any + Send + Sync> = Box::new(MutPtr(ptr));
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // two args - retrurn owned
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty:ty, ResultOp::Owned), $args_ty_plain1:ty, $args_ty_plain2:ty, $args_ty1:ty, $args_ty2:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty1, $args_ty2) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>){
                let mut arg2 = args.pop().unwrap();
                let mut arg1 = args.pop().unwrap();
                let w1 = arg1.get_arg::<$args_ty_plain1>();
                let w2 = arg2.get_arg::<$args_ty_plain2>();
                let result = (self.ptr)(w1.extract(), w2.extract());
                let serialised = result.tagged_string();
                let boxed: Box<dyn Any + Send + Sync> = Box::new(result);
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };

    // three args - return ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty_plain:ty, $res_ty:ty, ResultOp::Ref), $args_ty_plain1:ty, $args_ty_plain2:ty, $args_ty_plain3:ty, $args_ty1:ty, $args_ty2:ty, $args_ty3:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty1, $args_ty2, $args_ty3) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>){
                let mut arg3 = args.pop().unwrap();
                let mut arg2 = args.pop().unwrap();
                let mut arg1 = args.pop().unwrap();
                let w1 = arg1.get_arg::<$args_ty_plain1>();
                let w2 = arg2.get_arg::<$args_ty_plain2>();
                let w3 = arg3.get_arg::<$args_ty_plain3>();
                let result = (self.ptr)(w1.extract(), w2.extract(), w3.extract());
                let serialised = result.tagged_string();
                let ptr: *const $res_ty_plain = result;
                let boxed: Box<dyn Any + Send + Sync> = Box::new(ConstPtr(ptr));
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // three args - return mut ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty_plain:ty, $res_ty:ty, ResultOp::MutRef), $args_ty_plain1:ty, $args_ty_plain2:ty, $args_ty_plain3:ty, $args_ty1:ty, $args_ty2:ty, $args_ty3:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty1, $args_ty2, $args_ty3) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>){
                let mut arg3 = args.pop().unwrap();
                let mut arg2 = args.pop().unwrap();
                let mut arg1 = args.pop().unwrap();
                let w1 = arg1.get_arg::<$args_ty_plain1>();
                let w2 = arg2.get_arg::<$args_ty_plain2>();
                let w3 = arg3.get_arg::<$args_ty_plain3>();
                let result = (self.ptr)(w1.extract(), w2.extract(), w3.extract());
                let serialised = result.tagged_string();
                let ptr: *mut $res_ty_plain = result;
                let boxed: Box<dyn Any + Send + Sync> = Box::new(MutPtr(ptr));
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // three args - retrurn owned
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty:ty, ResultOp::Owned), $args_ty_plain1:ty, $args_ty_plain2:ty, $args_ty_plain3:ty, $args_ty1:ty, $args_ty2:ty, $args_ty3:ty) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn($args_ty1, $args_ty2, $args_ty3) -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, mut args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>){
                let mut arg3 = args.pop().unwrap();
                let mut arg2 = args.pop().unwrap();
                let mut arg1 = args.pop().unwrap();
                let w1 = arg1.get_arg::<$args_ty_plain1>();
                let w2 = arg2.get_arg::<$args_ty_plain2>();
                let w3 = arg3.get_arg::<$args_ty_plain3>();
                let result = (self.ptr)(w1.extract(), w2.extract(), w3.extract());
                let serialised = result.tagged_string();
                let boxed: Box<dyn Any + Send + Sync> = Box::new(result);
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // zero arg - return ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty_plain:ty, $res_ty:ty, ResultOp::Ref)) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn() -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>) {
                let result = (self.ptr)();
                let serialised = result.tagged_string();
                let ptr: *const $res_ty_plain = result;
                let boxed: Box<dyn Any + Send + Sync> = Box::new(ConstPtr(ptr));
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // zero arg - return mut ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty_plain:ty, $res_ty:ty, ResultOp::MutRef)) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn() -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>) {
                let result = (self.ptr)();
                let serialised = result.tagged_string();
                let ptr: *mut $res_ty_plain = result;
                let boxed: Box<dyn Any + Send + Sync> = Box::new(MutPtr(ptr));
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    };
    // zero arg - return non-ref
    ($map:ident, $name:ident, $fn_name:path, $fn_ty:ty, ($res_ty:ty, ResultOp::Owned)) => {
        #[derive(Clone)]
        pub struct $name {
            ptr: fn() -> $res_ty,
        }

        impl GenCall for $name {
            fn call(&self, args: Vec<Argument>) -> ((String, bool), Box<dyn Any + Send + Sync>) {
                let result = (self.ptr)();
                let serialised = result.tagged_string();
                let boxed: Box<dyn Any + Send + Sync> = Box::new(result);
                return (serialised, boxed);
            }
        }
        $map.insert(fn_type_name(&$fn_name), Box::new($name {ptr: ($fn_name as $fn_ty)}))
    }
}