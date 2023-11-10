use std::hint::black_box;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration, SystemTime};
use std::thread;
use umi::endpoint::{UMIEndpoint, ResourceTable};
use umi::{register, remote};
use applications::reminder::ready_reminder_server_multi::{ReadyReminderServer, Entry};
use umi_macros_proc::{setup_packages, setup_registry, setup_proc_macros};
setup_packages!();
setup_registry!();
setup_proc_macros!();

const SAMPLES:u64 = 1000;
const ROUNDS:u64 = 100;
const warmup:u64 = 10;


fn bench<F>(name: &str, mut f: F) where
    F: FnMut() -> () {
    for i in 0..warmup {
        f();
    }
    let then = Instant::now();
    for i in 0..SAMPLES {
        f();
    }

    let duration = then.elapsed();
    println!("{},{},{}", name, duration.as_millis(),SAMPLES);
}
fn server_setup() -> (UMIEndpoint, RegistryTable, Arc<Mutex<ResourceTable>>){
    let mut table = RegistryTable::new();
    register!(table, ReadyReminderServerNew, ReadyReminderServer::new, fn() -> ReadyReminderServer, (ReadyReminderServer, ResultOp::Owned));
    register!(table, ReadyReminderServerSubmit, ReadyReminderServer::submit_event, fn(&mut ReadyReminderServer, String, SystemTime), ((), ResultOp::Owned), ReadyReminderServer, String, SystemTime, &mut ReadyReminderServer, String, SystemTime);
    register!(table, ReadyReminderServerExtract, ReadyReminderServer::extract_event, fn(&mut ReadyReminderServer) -> Option<Entry>, (Option<Entry>, ResultOp::Owned), ReadyReminderServer, &mut ReadyReminderServer);

    let mut server = UMIEndpoint::new("127.0.0.1:3335");
    let vtable = Arc::new(Mutex::new(ResourceTable::new()));
    return (server, table, vtable)
}

fn server_benchmark() {
    bench("server setup", ||  {
        let _ = server_setup();
    });
}

fn client_benchmark() {
    let (mut server, table, vtable) = server_setup();
    let t = thread::spawn(move || {
        server.start(table, vtable);
    });
    let mut r = remote!("127.0.0.1:3335", ReadyReminderServer::new, ReadyReminderServer);
    bench("client submit",|| r.submit_event("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0)));
    //r.submit_event("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0));
    thread::sleep(Duration::new(4, 0));
    // r.extract_event();
    bench("client extract",|| {
        r.extract_event();
    });
    thread::sleep(Duration::new(2,0));
}

// fn extract_benchmark(c: &mut Criterion) {
//     let mut group = c.benchmark_group("client group");
//     group.measurement_time(Duration::from_millis(100));
//     let (mut server, table, vtable) = server_setup();
//     let t = thread::spawn(move || {
//         server.start(table, vtable);
//     });
//     let mut r = remote!("127.0.0.1:3335", ReadyReminderServer::new, ReadyReminderServer);
//     // group.bench_function("client submit", |b| b.iter(|| r.submit_event("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0))));
//     // thread::sleep(Duration::new(4, 0));
//     //r.extract_event();
//     group.bench_function("client extract", |b| b.iter(|| r.extract_event()));
//     group.finish();
// }

fn main() {
    println!("benchmark,total,samples");
    //server_benchmark();
    client_benchmark();
}