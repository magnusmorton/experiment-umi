use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use umi::endpoint::{UMIEndpoint, ResourceTable};
use umi::{register};
use applications::reminder::ready_reminder_server_multi::{ReadyReminderServer, Entry};
use umi_macros_proc::{setup_packages, setup_registry, setup_proc_macros};
setup_packages!();
setup_registry!();
setup_proc_macros!();

fn main() {
    let mut table = RegistryTable::new();
    register!(table, ReadyReminderServerNew, ReadyReminderServer::new, fn() -> ReadyReminderServer, (ReadyReminderServer, ResultOp::Owned));
    register!(table, ReadyReminderServerSubmit, ReadyReminderServer::submit_event, fn(&mut ReadyReminderServer, String, SystemTime), ((), ResultOp::Owned), ReadyReminderServer, String, SystemTime, &mut ReadyReminderServer, String, SystemTime);
    register!(table, ReadyReminderServerExtract, ReadyReminderServer::extract_event, fn(&mut ReadyReminderServer) -> Option<Entry>, (Option<Entry>, ResultOp::Owned), ReadyReminderServer, &mut ReadyReminderServer);

    let mut server = UMIEndpoint::new("127.0.0.1:3335");
    let vtable = Arc::new(Mutex::new(ResourceTable::new()));
    server.start(table, vtable);
}