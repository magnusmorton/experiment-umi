use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, Duration};
use std::thread;
use umi::endpoint::{UMIEndpoint, ResourceTable};
use umi::{register, remote};
use applications::reminder::ready_reminder_server_multi::{ReadyReminderServer, Entry};
use umi_macros_proc::{setup_packages, setup_registry, setup_proc_macros};
setup_packages!();
setup_registry!();
setup_proc_macros!();

fn fib(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fib(n-1) + fib(n-2),
    }
}

fn server_setup() -> (){
    let mut table = RegistryTable::new();
    register!(table, ReadyReminderServerNew, ReadyReminderServer::new, fn() -> ReadyReminderServer, (ReadyReminderServer, ResultOp::Owned));
    register!(table, ReadyReminderServerSubmit, ReadyReminderServer::submit_event, fn(&mut ReadyReminderServer, String, SystemTime), ((), ResultOp::Owned), ReadyReminderServer, String, SystemTime, &mut ReadyReminderServer, String, SystemTime);
    register!(table, ReadyReminderServerExtract, ReadyReminderServer::extract_event, fn(&mut ReadyReminderServer) -> Option<Entry>, (Option<Entry>, ResultOp::Owned), ReadyReminderServer, &mut ReadyReminderServer);
}

fn fib_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fib(black_box(20))));
}

fn server_benchmark(c: &mut Criterion) {
    c.bench_function("server setup", |b| b.iter(|| black_box(server_setup())));
}

fn client_benchmark(c: &mut Criterion) {
    let mut r = remote!("127.0.0.1:3335", ReadyReminderServer::new, ReadyReminderServer);
    c.bench_function("client submit", |b| b.iter(|| r.submit_event("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0))));
}

fn extract_benchmark(c: &mut Criterion) {
    let mut r = remote!("127.0.0.1:3335", ReadyReminderServer::new, ReadyReminderServer);
    let mut group = c.benchmark_group("client");
    group.throughput(criterion::Throughput::Elements(1));
    group.bench_function("client submit", |b| b.iter(|| r.submit_event("Goodbye World!".to_string(), SystemTime::now() + Duration::new(3, 0))));
    // thread::sleep(Duration::new(4, 0));
    //r.extract_event();
    group.bench_function("client extract", |b| b.iter(|| r.extract_event()));
}

criterion_group!(benches,  client_benchmark, extract_benchmark, server_benchmark);
criterion_main!(benches);