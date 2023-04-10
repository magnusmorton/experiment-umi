#![allow(unused_assignments)]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_quote};
use syn::{parse_macro_input};
use syn::visit_mut::{self, VisitMut};
use syn::visit::{self, Visit};

#[derive(Clone)]
enum ReturnTypeOptions {
    Default,
    Owned,
    Ref,
    MutRef
}

struct ExprVisitor {
    pub idents: Vec<syn::Ident>,
}

impl ExprVisitor {
    pub fn new() -> Self {
        ExprVisitor {
            idents: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for ExprVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        match node {
            syn::Expr::Field(f) => {
                match *(f.base) {
                    syn::Expr::Path(ref p) => {
                        if p.path.is_ident("self") {
                            match f.member {
                                syn::Member::Named(ref n) => {
                                    // gather invariants
                                    if !self.idents.contains(n) {
                                        self.idents.push(n.clone());
                                    }
                                },
                                _ => {}
                            }
                        } else {
                            // nothing need to be done
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }

        // Delegate to the default impl to visit any nested functions.
        visit::visit_expr(self, node);
    }
}


struct ExprReplace;

impl VisitMut for ExprReplace {
    fn visit_expr_mut(&mut self, node: &mut syn::Expr) {    
        // The pattern matching on self affects field access self.field -> Local{ref field}
        // all field accesses are changed into by reference/mutatble reference
        // in translation:
        // Replace &self.a with a
        // Replace self.a with *a
        match node {
            syn::Expr::Reference(r) => {
                match &*(r.expr) {
                    syn::Expr::Field(f) => {
                        match *(f.base) {
                            syn::Expr::Path(ref p) => {
                                if p.path.is_ident("self") {
                                    // replace &self.a with a
                                    match f.member {
                                        syn::Member::Named(ref n) => {
                                            let ident = n.clone();
                                            let new_expr = parse_quote! { #ident };
                                            *node = new_expr;
                                            return;
                                        },
                                        _ => {}
                                    }
                                } // else nothing change
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
            },
            syn::Expr::Field(f) => {
                match *(f.base) {
                    syn::Expr::Path(ref p) => {
                        if p.path.is_ident("self") {
                            // replace self.a with *a
                            match f.member {
                                syn::Member::Named(ref n) => {
                                    let ident = n.clone();
                                    let new_expr = parse_quote! { *#ident };
                                    *node = new_expr;
                                    return;
                                },
                                _ => {}
                            }
                        } // else nothing change
                    },
                    _ => {}
                }
            },
            _ => {}
        }

        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_expr_mut(self, node);
    }
}

#[proc_macro_derive(IsLocal)]
pub fn is_local_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_is_local(&ast)
}

fn impl_is_local(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl IsLocal for #name {
            fn is_local(&self) -> bool {
                match self {
                    Self::Remote{..} => { false },
                    _ => { true }
                }
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(ToVariable)]
pub fn to_variable_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_to_variable(&ast)
}

fn impl_to_variable(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl ToVariable for #name {
            fn to_variable(self) -> Variable {
                let var: Variable;
                if let #name::Remote(ref addr, ref id, is_owner) = &self {
                    let remote_borrow = #name::Remote(*addr, *id, Arc::new(AtomicBool::new(false)));
                    is_owner.swap(false, Ordering::Relaxed);
                    var = Variable::OwnedRemote(serde_json::to_string(&remote_borrow).unwrap(), *addr, *id);
                } else {
                    var = Variable::OwnedLocal(serde_json::to_string(&self).unwrap());
                }
                var
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(ToVariableRef)]
pub fn to_variable_ref_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_to_variable_ref(&ast)
}

fn impl_to_variable_ref(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl ToVariableRef for #name {
            fn to_variable(&self) -> Variable {
                let var: Variable;
                if let #name::Remote(ref addr, ref id, is_owner) = self {
                    let remote_borrow = #name::Remote(*addr, *id, Arc::new(AtomicBool::new(false)));
                    var = Variable::RefRemote(serde_json::to_string(&remote_borrow).unwrap(), *addr, *id);
                } else {
                    // This case should not happen
                    unimplemented!();
                }
                var
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(ToVariableMut)]
pub fn to_variable_mut_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_to_variable_mut(&ast)
}

fn impl_to_variable_mut(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl ToVariableMut for #name {
            fn to_variable(&mut self) -> Variable {
                let var: Variable;
                if let #name::Remote(ref addr, ref id, is_owner) = self {
                    let remote_borrow = #name::Remote(*addr, *id, Arc::new(AtomicBool::new(false)));
                    var = Variable::MutRefRemote(serde_json::to_string(&remote_borrow).unwrap(), *addr, *id);
                } else {
                    // This case should not happen
                    unimplemented!();
                }
                var
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(DropMarker)]
pub fn drop_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_drop(&ast)
}

fn impl_drop(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl Drop for #name {
            fn drop(&mut self) {
                match self {
                    Self::Remote(addr, id, is_owner) => {
                        if is_owner.load(Ordering::Relaxed) {
                            let msg = Message::Drop(*id);
                            send(*addr, msg).unwrap();
                        }
                    },
                    _ => {}
                }
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(SerializeTag)]
pub fn serialize_tag_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_serialize_tag(&ast)
}

fn impl_serialize_tag(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl SerializeTag for #name {
            fn tagged_string(&self) -> (String, bool) {
                let serialised = serde_json::to_string(&self).unwrap();
                match self {
                    Self::Local{..} => {
                        return (serialised, true)
                    },
                    _ => {
                        return (serialised, false)
                    }
                }
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(BorrowRemoteMarker)]
pub fn borrow_remote_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_borrow_remote(&ast)
}

fn impl_borrow_remote(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl BorrowRemote for #name {
            fn borrow_remote(&self) -> Self {
                match self {
                    Self::Local{..} => { panic!("Only suitable for remote"); },
                    Self::Remote(addr, id, _) => {
                        Self::Remote(*addr, *id, Arc::new(AtomicBool::new(false)))
                    }
                }
            }
        }
    };
    gen.into()
}


#[proc_macro_derive(IsProxyType, attributes(is_lifted_or_not))]
pub fn is_proxy_type_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_is_proxy_type(&ast)
}
fn impl_is_proxy_type(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let attrs = &ast.attrs;
    if attrs.len() != 1 {
        panic!("Expected exactly one attribute");
    } else {
        let attr = &attrs[0];
        if attr.path.segments.len() > 1 {
            panic!("Expected exactly one attribute");
        } else {
            let attr_ident = &attr.path.segments[0].ident;
            if attr_ident != "is_lifted_or_not" {
                panic!("Expected attribute to be is_lifted_or_not");
            } else {
                let attr_tokens = &attr.tokens;
                let attr_tokens_str = attr_tokens.to_string();
                if attr_tokens_str != "(lifted)" && attr_tokens_str != "(not_lifted)" {
                    panic!("Expected attribute to be either (lifted) or (not_lifted)");
                } else if attr_tokens_str == "(lifted)" {
                    let gen = quote! {
                        impl IsProxyType for #name {
                            fn is_proxy_type(&self) -> bool {
                                true
                            }
                        }
                    };
                    return gen.into();
                } else {
                    let gen = quote! {
                        impl IsProxyType for #name {
                            fn is_proxy_type(&self) -> bool {
                                false
                            }
                        }
                    };
                    return gen.into();
                }
            }
        }
    }
}

#[proc_macro_attribute]
pub fn proxy_me(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut ty = parse_macro_input!(input as syn::Item);
    assert!(args.is_empty());

    match &mut ty {
        syn::Item::Struct(s) => {
            let struct_ident = s.ident.clone();
            let visibility = s.vis.clone();
            let fields = s.fields.clone();
            let mut field_tys: Vec<syn::Type> = Vec::new();
            let mut field_names: Vec<syn::Ident> = Vec::new();
            for f in fields.iter() {
                field_tys.push(f.ty.clone());
                field_names.push(f.ident.clone().unwrap());
            }
            let gen = quote! {
                #[derive(IsLocal, ToVariable, ToVariableRef, ToVariableMut, Serialize, Deserialize, Clone, DropMarker, IsProxyType, SerializeTag, BorrowRemoteMarker)]
                #[is_lifted_or_not(lifted)]
                #visibility enum #struct_ident {
                    Local{#(#field_names: #field_tys),*},
                    Remote(SocketAddr, ID, Arc<AtomicBool>)
                }
            };
            ty = syn::parse(gen.into()).unwrap();
            ty.into_token_stream().into()
        },
        syn::Item::Enum(ref mut e) => {
            let gen = quote! { Remote(SocketAddr, ID, Arc<AtomicBool>) };
            e.variants.push(syn::parse(gen.into()).unwrap());
            let result = quote!{
                #[derive(IsLocal, ToVariable, ToVariableRef, ToVariableMut, Serialize, Deserialize, Clone)]
                #ty
            };
            result.into_token_stream().into()
        },
        _ => {unimplemented!{}}
    }
}

#[proc_macro_attribute]
pub fn umi_init(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut func_block = parse_macro_input!(input as syn::Item);
    assert!(args.is_empty());

    match &func_block {
        syn::Item::Fn(func) => {
            let mut pos = 0;
            let mut gen = None;
            for stmt in func.block.stmts.iter() {
                match stmt {
                    syn::Stmt::Expr(expr) => {
                        match expr {
                            syn::Expr::Struct(s) => {
                                let rty = s.path.segments[0].ident.clone();
                                let mut args: Vec<syn::Expr> = Vec::new();
                                let mut field_names: Vec<syn::Ident> = Vec::new();
                                for f in s.fields.iter() {
                                    match &f.member {
                                        syn::Member::Named(ident) => {
                                            field_names.push(ident.clone());
                                        },
                                        _ => {unimplemented!{}}
                                    }
                                    match &f.expr {
                                        syn::Expr::Path(..) => {
                                            args.push(f.expr.clone());
                                        },
                                        syn::Expr::Call(..) => {
                                            args.push(f.expr.clone());
                                        },
                                        _ => {
                                            // maybe there are more possible expressions?
                                            panic!("umi_init error: Not a path type")
                                        }
                                    }
                                }
                                let gen_stmt = quote! {
                                    #rty::Local{#(#field_names: #args),*}
                                };
                                gen = Some(gen_stmt);
                                break;
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
                pos += 1;
            }
            match &mut func_block {
                syn::Item::Fn(ref mut func) => {
                    match gen {
                        Some(g) => {
                            // replace the statement for constructing struct with the generated one
                            let expr: syn::Expr = syn::parse(g.into()).unwrap();
                            func.block.stmts[pos] = syn::Stmt::Expr(expr);
                            func_block.into_token_stream().into()
                        },
                        None => {panic!("umi_init error: Invalid generated stmt")}
                    }
                },
                _ => {panic!("umi_init error: Not a function")} // should not reach here
            }
        },
        _ => {panic!("umi_init error: Not a function")} // should not reach here
    }
}

// gnerating the deserialisation of the return value for the remote case
fn gen_remote_match_expr(op: ReturnTypeOptions, ident: Option<syn::TypePath>, return_type_lifted: bool) -> Option<syn::ExprMatch> {
    match op {
        ReturnTypeOptions::Default => {
            None
        },
        ReturnTypeOptions::Owned => {
            let ty = ident.unwrap();
            let gen;
            if return_type_lifted {
                gen = quote! {
                    match deserialised {
                        Message::Return(v) => {
                            match v {
                                ReturnVar::Owned(s) => {
                                    let result: #ty = serde_json::from_str(&s).unwrap();
                                    if let #ty::Remote(addr, id, _) = result {
                                        #ty::Remote(addr, id, Arc::new(AtomicBool::new(true)))
                                    } else {
                                        result
                                    }
                                },
                                _ => {panic!("Wrong return value")}
                            }
                        },
                        _ => {panic!("Invalid return message")}
                    }
                };
            } else {
                gen = quote! {
                    match deserialised {
                        Message::Return(v) => {
                            match v {
                                ReturnVar::Owned(s) => {
                                    let result: #ty = serde_json::from_str(&s).unwrap();
                                    result
                                },
                                _ => {panic!("Wrong return value")}
                            }
                        },
                        _ => {panic!("Invalid return message")}
                    }
                };
            }

            // TODO here -- change the Option as an extra tag
            let expr : syn::ExprMatch = syn::parse(gen.into()).unwrap();
            Some(expr)
        },
        ReturnTypeOptions::Ref => {
            let ty = ident.unwrap();
            let gen = quote! {
                match deserialised {
                    Message::Return(v) => {
                        match v {
                            ReturnVar::RefOwned(addr, id)=> {
                                let remote: Box<dyn Any> = Box::new(#ty::Remote(addr, id, Arc::new(AtomicBool::new(true))));
                                unsafe {
                                    REFS.push(remote); // hold the value in global varible for a longer lifetime
                                    REFS.last().unwrap().downcast_ref::<#ty>().unwrap()
                                }
                            },
                            ReturnVar::RefBorrow(serialised) => {
                                let deserialised: #ty = serde_json::from_str(&serialised).unwrap();
                                if let #ty::Remote(addr, id, _) = deserialised {
                                    let remote: Box<dyn Any> = Box::new(#ty::Remote(addr, id, Arc::new(AtomicBool::new(false))));
                                    unsafe {
                                        REFS.push(remote); // hold the value in global varible for a longer lifetime
                                        REFS.last().unwrap().downcast_ref::<#ty>().unwrap()
                                    }
                                } else {
                                    panic!("Wrong return value")
                                }
                            },
                            _ => {panic!("Wrong return value")}
                        }
                    },
                    _ => {panic!("Invalid return message")}
                }
            };
            let expr: syn::ExprMatch = syn::parse(gen.into()).unwrap();
            Some(expr)
        },
        ReturnTypeOptions::MutRef => {
            let ty = ident.unwrap();
            let gen = quote!{
                match deserialised {
                    Message::Return(v) => {
                        match v {
                            ReturnVar::MutRefOwned(addr, id) => {
                                let remote: Box<dyn Any> = Box::new(#ty::Remote(addr, id, Arc::new(AtomicBool::new(true))));
                                unsafe {
                                    REFS.push(remote); // hold the value in global varible for a longer lifetime
                                    REFS.last_mut().unwrap().downcast_mut::<#ty>().unwrap()
                                }
                            },
                            ReturnVar::MutRefBorrow(serialised) => {
                                let deserialised: #ty = serde_json::from_str(&serialised).unwrap();
                                if let #ty::Remote(addr, id, _) = deserialised {
                                    let remote: Box<dyn Any> = Box::new(#ty::Remote(addr, id, Arc::new(AtomicBool::new(false))));
                                    unsafe {
                                        REFS.push(remote); // hold the value in global varible for a longer lifetime
                                        REFS.last_mut().unwrap().downcast_mut::<#ty>().unwrap()
                                    }
                                } else {
                                    panic!("Wrong return value 1")
                                }
                            },
                            _ => {panic!("Wrong return value")}
                        }
                    },
                    _ => {panic!("Invalid return message")}
                }
            };
            let expr: syn::ExprMatch = syn::parse(gen.into()).unwrap();
            Some(expr)
        },
    }
}

#[proc_macro_attribute]
pub fn umi_struct_method(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut func_block = parse_macro_input!(input as syn::Item);
    let mut func_block_clone = func_block.clone();
    let func_block_clone_imm = func_block.clone();
    //assert!(args.is_empty());
    let mut expr_visitor = ExprVisitor::new();
    let mut return_type = ReturnTypeOptions::Default;
    let mut return_type_ident: Option<syn::TypePath> = None;
    let mut func_ident: Option<syn::Ident> = None;
    match &func_block_clone_imm {
        syn::Item::Fn(ref func) => {
            let output = &func.sig.output;
            func_ident = Some(func.sig.ident.clone()); // get the function name, it should not reach the None case, unless the function is not named
            match output {
                syn::ReturnType::Default => {},
                syn::ReturnType::Type(_, ref ty) => {
                    match **ty {
                        syn::Type::Path(ref tp) => {
                            return_type = ReturnTypeOptions::Owned;
                            return_type_ident = Some(tp.clone());
                        },
                        syn::Type::Reference(ref r) => {
                            match *r.elem {
                                syn::Type::Path(ref tp) => {
                                    return_type_ident = Some(tp.clone());
                                },
                                _ => {
                                    unimplemented!(); // the reference type might be other options
                                }
                            }
                            match r.mutability {
                                Some(_) => {
                                    return_type = ReturnTypeOptions::MutRef;
                                },
                                None => {
                                    return_type = ReturnTypeOptions::Ref;
                                }
                            }
                        },
                        _ => {
                            unimplemented!(); // might be other types ?
                        }
                    }
                }
            }
        },
        _ => {panic!("umi_method error: Not a function")} // should not reach here
    }
    let mut return_lifted = true;
    if !args.is_empty() {
        if args.to_string() == "false" {
            return_lifted = false;
        }
    }
    let match_expr = gen_remote_match_expr(return_type.clone(), return_type_ident, return_lifted);
    
    if match_expr.is_some() { // has return value
        let match_expr = match_expr.unwrap();
        match &mut func_block_clone {
            syn::Item::Fn(ref mut func) => {
                expr_visitor.visit_item_fn(func);
                let invariants = expr_visitor.idents;
                ExprReplace.visit_item_fn_mut(func);
                let stmts = &func.block.stmts;
                let func_ident = func_ident.unwrap(); // it is safe to unwrap here
                // Processing arguments for the function
                let inputs = &func.sig.inputs;
                let mut args_gen = Vec::new();
                let mut is_mut_self = false;
                for input in inputs {
                    match input {
                        syn::FnArg::Receiver(ref r) => { //self
                            match r.mutability {
                                Some(_) => {
                                    let gen = quote! {
                                        Variable::MutRefRemote(serde_json::to_string(&self).unwrap(), *addr, *id)
                                    };
                                    args_gen.push(gen);
                                    is_mut_self = true; // self is mutable
                                },
                                None => {
                                    let gen = quote! {
                                        Variable::RefRemote(serde_json::to_string(&self).unwrap(), *addr, *id)
                                    };
                                    args_gen.push(gen);
                                }
                            }
                        },
                        syn::FnArg::Typed(ref pat) => { //other than self
                            match *pat.pat {
                                syn::Pat::Ident(ref ident) => {
                                    let ident = &ident.ident;
                                    let gen = quote! {
                                        #ident.to_variable()
                                    };
                                    args_gen.push(gen);
                                },
                                _ => {// should not reach here
                                    unimplemented!();
                                }
                            }
                        }
                    }
                }
                // invoke op
                let op;
                match return_type {
                    ReturnTypeOptions::Owned | ReturnTypeOptions::Default => {
                        op = quote! {InvokeOp::Owned};
                    },
                    ReturnTypeOptions::Ref => {
                        op = quote! {InvokeOp::Ref};
                    },
                    ReturnTypeOptions::MutRef => {
                        op = quote! {InvokeOp::MutRef};
                    }
                }
                let gen;
                if is_mut_self {
                    gen = quote! {
                        match self {
                            Self::Local{#(ref mut #invariants),* , ..} => {
                                #(#stmts)*
                            },
                            Self::Remote(ref addr, ref id, is_owner) => {
                                let msg = Message::Invoke(fn_type_name(&Self::#func_ident).to_string(), 
                                vec![#(#args_gen),*], #op);
                                let result_msg = send(addr, msg).unwrap();
                                //println!("{:?}", result_msg);
                                let deserialised: Message = serde_json::from_str(&*result_msg).unwrap();
                                #match_expr
                            }
                        }
                    };
                } else {
                    gen = quote! {
                        match self {
                            Self::Local{#(ref #invariants),* , ..} => {
                                #(#stmts)*
                            },
                            Self::Remote(ref addr, ref id, is_owner) => {
                                let msg = Message::Invoke(fn_type_name(&Self::#func_ident).to_string(),
                                vec![#(#args_gen),*], #op);
                                let result_msg = send(addr, msg).unwrap();
                                //println!("{:?}", result_msg);
                                let deserialised: Message = serde_json::from_str(&*result_msg).unwrap();
                                #match_expr
                            }
                        }
                    };
                }
                
                let expr: syn::Expr = syn::parse(gen.into()).unwrap();
                match &mut func_block {
                    syn::Item::Fn(ref mut func) => {
                        func.block.stmts = vec![syn::Stmt::Expr(expr)];
                        func_block.into_token_stream().into()
                    },
                    _ => {panic!("umi_method error: Not a function")} // should not reach here
                }
            },
            _ => {panic!("umi_init error: Not a function")} // should not reach here
        }   
    } else { // no return value
        match &mut func_block_clone {
            syn::Item::Fn(ref mut func) => {
                expr_visitor.visit_item_fn(func);
                let invariants = expr_visitor.idents;
                ExprReplace.visit_item_fn_mut(func);
                let stmts = &func.block.stmts;
                // Processing arguments for the function
                let inputs = &func.sig.inputs;
                let mut args_gen = Vec::new();
                for input in inputs {
                    match input {
                        syn::FnArg::Receiver(ref r) => {
                            match r.mutability {
                                Some(_) => {
                                    let gen = quote! {
                                        Variable::MutRefRemote(serde_json::to_string(&self).unwrap(), *addr, *id)
                                    };
                                    args_gen.push(gen);
                                },
                                None => {
                                    let gen = quote! {
                                        Variable::RefRemote(serde_json::to_string(&self).unwrap(), *addr, *id)
                                    };
                                    args_gen.push(gen);
                                }
                            }
                        },
                        syn::FnArg::Typed(ref pat) => {
                            match *pat.pat {
                                syn::Pat::Ident(ref ident) => {
                                    let ident = &ident.ident;
                                    let gen = quote! {
                                        #ident.to_variable()
                                    };
                                    args_gen.push(gen);
                                },
                                _ => {// should not reach here
                                    unimplemented!();
                                }
                            }
                        }
                    }
                }
                let gen = quote! {
                    match self {
                        Self::Local{#(#invariants),* , ..} => {
                            #(#stmts)*
                        },
                        Self::Remote(ref addr, ref id, is_owner) => {
                            //println!("here invoking");
                            let msg = Message::Invoke(fn_type_name(&Self::#func_ident).to_string(), 
                            vec![#(#args_gen),*], InvokeOp::Owned);
                            send(addr, msg).unwrap();
                        }
                    }
                };
                let expr: syn::Expr = syn::parse(gen.into()).unwrap();
                match &mut func_block {
                    syn::Item::Fn(ref mut func) => {
                        func.block.stmts = vec![syn::Stmt::Expr(expr)];
                        func_block.into_token_stream().into()
                    },
                    _ => {panic!("umi_method error: Not a function")} // should not reach here
                }
            },
            _ => {panic!("umi_init error: Not a function")} // should not reach here
        }
    }
}

#[proc_macro_attribute]
pub fn umi_enum_method(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut func_block = parse_macro_input!(input as syn::Item);
    let mut func_block_clone = func_block.clone();
    let func_block_clone_imm = func_block.clone();
    assert!(args.is_empty());
    let mut expr_visitor = ExprVisitor::new();
    let mut return_type = ReturnTypeOptions::Default;
    let mut return_type_ident: Option<syn::TypePath> = None;
    let mut func_ident: Option<syn::Ident> = None;
    match &func_block_clone_imm {
        syn::Item::Fn(ref func) => {
            let output = &func.sig.output;
            func_ident = Some(func.sig.ident.clone()); // get the function name, it should not reach the None case, unless the function is not named
            match output {
                syn::ReturnType::Default => {},
                syn::ReturnType::Type(_, ref ty) => {
                    match **ty {
                        syn::Type::Path(ref tp) => {
                            return_type = ReturnTypeOptions::Owned;
                            // return_type_ident = Some(tp.path.segments[0].ident.clone());
                            return_type_ident = Some(tp.clone());
                        },
                        syn::Type::Reference(ref r) => {
                            match *r.elem {
                                syn::Type::Path(ref tp) => {
                                    //return_type_ident = Some(tp.path.segments[0].ident.clone());
                                    return_type_ident = Some(tp.clone());
                                },
                                _ => {
                                    unimplemented!(); // the reference type might be other options
                                }
                            }
                            match r.mutability {
                                Some(_) => {
                                    return_type = ReturnTypeOptions::MutRef;
                                },
                                None => {
                                    return_type = ReturnTypeOptions::Ref;
                                }
                            }
                        },
                        _ => {
                            unimplemented!(); // might be other types ?
                        }
                    }
                }
            }
        },
        _ => {panic!("umi_method error: Not a function")} // should not reach here
    }
    let mut return_lifted = true;
    if !args.is_empty() {
        if args.to_string() == "false" {
            return_lifted = false;
        }
    }
    let match_expr = gen_remote_match_expr(return_type.clone(), return_type_ident, return_lifted);

    if match_expr.is_some() { // has return value
        let match_expr = match_expr.unwrap();
        match &mut func_block_clone {
            syn::Item::Fn(ref mut func) => {
                let func_ident = func_ident.unwrap(); // it is safe to unwrap here
                // Processing arguments for the function
                let inputs = &func.sig.inputs;
                let mut args_gen = Vec::new();
                let mut is_mut_self = false;
                for input in inputs {
                    match input {
                        syn::FnArg::Receiver(ref r) => { //self
                            match r.mutability {
                                Some(_) => {
                                    let gen = quote! {
                                        Variable::MutRefRemote(serde_json::to_string(&self).unwrap(), *addr, *id)
                                    };
                                    args_gen.push(gen);
                                    is_mut_self = true; // self is mutable
                                },
                                None => {
                                    let gen = quote! {
                                        Variable::RefRemote(serde_json::to_string(&self).unwrap(), *addr, *id)
                                    };
                                    args_gen.push(gen);
                                }
                            }
                        },
                        syn::FnArg::Typed(ref pat) => { //other than self
                            match *pat.pat {
                                syn::Pat::Ident(ref ident) => {
                                    let ident = &ident.ident;
                                    let gen = quote! {
                                        #ident.to_variable()
                                    };
                                    args_gen.push(gen);
                                },
                                _ => {// should not reach here
                                    unimplemented!();
                                }
                            }
                        }
                    }
                }
                // invoke op
                let op;
                match return_type {
                    ReturnTypeOptions::Owned | ReturnTypeOptions::Default => {
                        op = quote! {InvokeOp::Owned};
                    },
                    ReturnTypeOptions::Ref => {
                        op = quote! {InvokeOp::Ref};
                    },
                    ReturnTypeOptions::MutRef => {
                        op = quote! {InvokeOp::MutRef};
                    }
                }
                let gen;
                if is_mut_self {
                    gen = quote! {
                        Self::Remote(ref addr, ref id, is_owner) => {
                            let msg = Message::Invoke(fn_type_name(&Self::#func_ident).to_string(), 
                            vec![#(#args_gen),*], #op);
                            let result_msg = send(addr, msg).unwrap();
                            //println!("{:?}", result_msg);
                            let deserialised: Message = serde_json::from_str(&*result_msg).unwrap();
                            #match_expr
                        }
                    };
                } else {
                    gen = quote! {
                        Self::Remote(ref addr, ref id, is_owner) => {
                            let msg = Message::Invoke(fn_type_name(&Self::#func_ident).to_string(),
                            vec![#(#args_gen),*], #op);
                            let result_msg = send(addr, msg).unwrap();
                            //println!("{:?}", result_msg);
                            let deserialised: Message = serde_json::from_str(&*result_msg).unwrap();
                            #match_expr
                        }
                    };
                }
                
                let arm: syn::Arm = syn::parse(gen.into()).unwrap();
                match &mut func_block {
                    syn::Item::Fn(ref mut func) => {
                        // add a match arm to the match expression matching on self in the stmts
                        for stmt in func.block.stmts.iter_mut() {
                            match stmt {
                                syn::Stmt::Expr(ref mut expr) => {
                                    match expr {
                                        syn::Expr::Match(ref mut m) => {
                                            match *m.expr {
                                                syn::Expr::Path(ref mut p) => {
                                                    if p.path.is_ident("self") {
                                                        m.arms.push(arm.clone());
                                                    } // only add the arm if the match expression is self
                                                },
                                                _ => {} // do nothing
                                            }
                                            
                                        },
                                        _ => {} // do nothing
                                    }
                                },
                                _ => {} // do nothing
                            }
                        }
                        func_block.into_token_stream().into()
                    },
                    _ => {panic!("umi_method error: Not a function")} // should not reach here
                }
            },
            _ => {panic!("umi_init error: Not a function")} // should not reach here
        }   
    } else { // no return value
        match &mut func_block_clone {
            syn::Item::Fn(ref mut func) => {
                expr_visitor.visit_item_fn(func);
                ExprReplace.visit_item_fn_mut(func);
                // Processing arguments for the function
                let inputs = &func.sig.inputs;
                let mut args_gen = Vec::new();
                for input in inputs {
                    match input {
                        syn::FnArg::Receiver(ref r) => {
                            match r.mutability {
                                Some(_) => {
                                    let gen = quote! {
                                        Variable::MutRefRemote(serde_json::to_string(&self).unwrap(), *addr, *id)
                                    };
                                    args_gen.push(gen);
                                },
                                None => {
                                    let gen = quote! {
                                        Variable::RefRemote(serde_json::to_string(&self).unwrap(), *addr, *id)
                                    };
                                    args_gen.push(gen);
                                }
                            }
                        },
                        syn::FnArg::Typed(ref pat) => {
                            match *pat.pat {
                                syn::Pat::Ident(ref ident) => {
                                    let ident = &ident.ident;
                                    let gen = quote! {
                                        #ident.to_variable()
                                    };
                                    args_gen.push(gen);
                                },
                                _ => {// should not reach here
                                    unimplemented!();
                                }
                            }
                        }
                    }
                }
                let gen = quote! {
                    Self::Remote(ref addr, ref id, is_owner) => {
                        Message::Invoke(fn_type_name(&Self::#func_ident).to_string(), 
                        vec![#(#args_gen),*], InvokeOp::Owned);
                    }
                };
                
                let arm: syn::Arm = syn::parse(gen.into()).unwrap();
                match &mut func_block {
                    syn::Item::Fn(ref mut func) => {
                        // add a match arm to the match expression matching on self in the stmts
                        for stmt in func.block.stmts.iter_mut() {
                            match stmt {
                                syn::Stmt::Expr(ref mut expr) => {
                                    match expr {
                                        syn::Expr::Match(ref mut m) => {
                                            match *m.expr {
                                                syn::Expr::Path(ref mut p) => {
                                                    if p.path.is_ident("self") {
                                                        m.arms.push(arm.clone());
                                                    } // only add the arm if the match expression is self
                                                },
                                                _ => {} // do nothing
                                            }
                                            
                                        },
                                        _ => {} // do nothing
                                    }
                                },
                                _ => {} // do nothing
                            }
                        }
                        func_block.into_token_stream().into()
                    },
                    _ => {panic!("umi_method error: Not a function")} // should not reach here
                }
            },
            _ => {panic!("umi_init error: Not a function")} // should not reach here
        }
    }
}

// imports
#[proc_macro]
pub fn setup_packages(_item: TokenStream) -> TokenStream {
    "use std::sync::atomic::*;
    use std::sync::*;
    use umi::utils::*;
    use umi::message_serialisation::*;".parse().unwrap()
}

#[proc_macro]
pub fn setup_registry(_item: TokenStream) -> TokenStream {
    "use std::any::*;
    use umi::registry::*;
    use umi::proxy_lib::*;".parse().unwrap()
}

#[proc_macro]
pub fn setup_proc_macros(_item: TokenStream) -> TokenStream {
    "use serde::*;
    use std::net::*;
    use umi_macros::*;
    use umi_macros_proc::*;".parse().unwrap()
}