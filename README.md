# Experiment UMI
This repository is prepared for evaluating the effort of using our UMI library to refactor an application that runs on a single nodes into an application that runs on multiple nodes.

## Application Directory and Library Directory
- All application code to be worked on for this experiment are located in the directory: `./applications/`. Please only modify code under this directory.
    - `./applications/examples/` contains application code to be executed.
    - `./applications/src/` contains stucts definitions for single and mutiple node applications.
- Directories `./src/` and `./umi_macros/` contains UMI library code. Please do not modify code under these directories.

## Getting Started
- Forking the repository.
- Cloning the forked repository into your own machine.
- Make sure you are under the directory `./applications/`
- Running all tests: `cargo test`
    - You should see the following output:
    ```
    running 6 tests
    test reminder::ready_reminder_server::tests::gt_works ... ok
    test reminder::ready_reminder_server::tests::cmp_works ... ok
    test reminder::ready_reminder_server::tests::heap_works ... ok
    test reminder::ready_reminder_server::tests::can_submit ... ok
    test umi_demo::students::tests::student_record_works ... ok
    test umi_demo::students_single::tests::student_single_record_works ... ok
    ```
- Executing the provided single node application: `cargo run --example ready_reminder_client`
    - You should see the following output:
    ```
    The first event is: None
    The first event is: Some(Entry { content: "Hello World!", time: SystemTime { tv_sec: 1681138028, tv_nsec: 61158000 } })
    The first event is: Some(Entry { content: "Goodbye World!", time: SystemTime { tv_sec: 1681138030, tv_nsec: 61153000 } })
    ```
## An example application using UMI
- In there directory `./application/src/umi_demo/`, there are two implementations of a simple student record storage.
    - `students_single.rs` is suitable for a single-node application
    - `students.rs` is suitable for a multiple-node application
- A single node application `student_record_single.rs` is located in the directory `./applications/`, implemented using structs defined in `students_single.rs`. Executing it with command `cargo run --example student_record_single` should give you the following output:
    ```
    Has student John Doe? : false
    Has student Jane Doe? : true
    ```
- In the file `students.rs`, an example of writing a simple student record storage with UMI is provided.
    - It is basically written by annotating the exisiting code `students_single.rs` with additional UMI macros (and importing crates that are used by those macros). These macros make it suitable for applications running on multiple nodes.
    - There is a server (`student_record_server.rs`) and a client (`student_record_client.rs`) in the directory `./applications/examples/`.
        - The server contains two tables, one is a `ResourceTable` to store data invovled in the invocation, the other one is a `RegistryTable` to store information of methods that can be invoked remotely. Those methods are register with the macro `register!(...)`.
        - The client is a modification of the single node application. Instead of storing student records locally, it stores and queries student records on a server.
        - To execute this server-client application, you will need two terminal instances. While running the server with `cargo run --example student_record_server` in one instance, you can run the client with `cargo run --example student_record_client` in the other instance. On the server side you will be able to see and log of incoming and outgoing messages and on the client side you should see the following output:
            ```
            Successfully connected to server
            Message Sent
            Successfully connected to server
            Message Sent
            Successfully connected to server
            Message Sent
            Has student John Doe? : false
            Successfully connected to server
            Message Sent
            Has student Jane Doe? : true
            Successfully connected to server
            Message Sent    
            ```

## Instructions of Using UMI Library
### UMI Structs
#### __`umi::endpoint::UMIEndpoint`__
```rust
pub struct UMIEndpoint {
    listener: TcpListener,
}
```
- Creating a new `UMIEndpoint` to listen and response to requests from clients:
```rust
pub fn new<A: ToSocketAddrs>(addr: A) -> UMIEndpoint
```
- Starting a `UMIEndpoint`:
```rust
pub fn start(&mut self, registry_table: RegistryTable, vtable: Arc<Mutex<ResourceTable>>)
```

#### __`umi::endpoint::ResourceTable`__
A `ResourceTable` is a type alias of a `std::collections::HashMap`.
```rust
pub type ResourceTable = HashMap<ID, RefCell<(Box<dyn Any + Send + Sync>, bool)>>;
```

#### __`umi::registry::RegistryTable`__
A `ResourceTable` is a type alias of a `std::collections::HashMap`.
```rust
pub type RegistryTable = HashMap<&'static str, Box<dyn GenCall>>;
```

### UMI Macros
#### __`umi::remote`__
`remote!(...)` is used as the entry point of an multiple-node application. A client can use it to send an request to a server to initialise tha allocation of some resources. It will return a proxy to the client than can be used to invoking computation on the the server if the initial allocation is successful.
```rust
remote!(addr, method_name, return_type);
```
An example usage is in `student_record_client.rs`:
```rust
let mut record = remote!("127.0.0.1:3334", StudentRecord::new, StudentRecord);
```
This creates a `StudentRecord` on the server with the address `127.0.0.1:3334`. It returns a proxy `record` to the client. On the client, methods can be directly invoked on this proxy. The actual computation will be sent to the server and the result of the computation will be sent back to the client.
#### __`umi::register`__
`register!(...)` requires `std::any::Any` to be imported. It is used to register methods in the `RegistryTable` for remote invocation.
```rust
register!(registry_table, registry_name, method_name, method_type_signature, (method_return_type, method_return_ownership), argument_types*, argument_ownerships*)
```
Example usages can be find in `student_record_server.rs`. For example, to register the `add_student` method of the strut `StudentRecord` into `table`(which is a`RegistryTable`) for remote invocations:
```rust
register!(table, // method registry table
    StudentRecordAdd, // method registry name
    StudentRecord::add_student, // method name
    fn(&mut StudentRecord, Student),  // method signature
    ((), ResultOp::Owned), // method return type and ownership
    StudentRecord, Student, // argument types
    &mut StudentRecord, String); // argument ownership
```
#### __`umi_macro_proc::proxy_me`__
`#[proxy_me]` makes a struct able to represent both local resouce and a proxy. An example usage is in `student.rs`:
```rust
#[proxy_me]
pub struct StudentRecord {
    students: Vec<Student>
}
```
#### __`umi_macro_proc::umi_init`__
`#[umi_init]` makes a initialisation call `new` able to be sent to a remote node and return a proxy to the local node. An example usage is in `student.rs`:
```rust
#[umi_init]
pub fn new() -> Self {
    StudentRecord {
        students: Vec::new()
    }
}
```
This allows `new()` to be send in the entry point:
```rust
let mut record = remote!("127.0.0.1:3334", StudentRecord::new, StudentRecord);
```

#### __`umi_macro_proc::umi_struct_method(option)`__
`#[umi_struct_method]` makes a struct method able to be also invoked on a proxy. `#[umi_struct_method(false)]` specifically indicates that the return value is sent back by copy/move and it does not have a proxy representation. An example usage is in `student.rs`:
```rust
#[umi_struct_method(false)]
pub fn has_student(&self, student: Student) -> bool {
    (&self.students).contains(&student)
}
```
This allows the method `has_student` to be invoked on a proxy `StudentRecord` on a client, and the boolean return value is sent back by copy and such boolean value does not have a proxy representation.

#### __`umi_macro_proc::setup_packages`__ 
`setup_packages!();` imports relevant hidden crates for message serialisation.

#### __`umi_macro_proc::setup_registry`__
`setup_registry!();` imports relevant hidden crates for method registry.

#### __`umi_macro_proc::setup_proc_macros`__
`setup_proc_macros!();` imports relevant hidden crates for using umi macros.

## Task
- Please refactor the single node reminder application `./applications/examples/ready_reminder_client.rs` using the UMI library to run on multiple nodes. The `ReadyReminderServer` should run on a node, accepting requests from client(s) nodes that submit or extract events.
    - You should first consider modifying `./applications/src/reminder/ready_reminder_server.rs`.

## Evaluation

Please work independently of the other participants.

Please keep track of the amount of time that you spend on various programming tasks (including but not limited to reading documentations, seeking support, design, implementation, testing, and debugging). For each task please record observations if pertinent (especially levels of satisfaction or frustration with the process).