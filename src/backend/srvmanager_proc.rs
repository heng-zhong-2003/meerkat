use crate::backend::message::{Message, Val};
use crate::{backend::worker::Worker, frontend::meerast, frontend::typecheck};
use inline_colorization::*;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

const BUFFER_SIZE: usize = 1024;

async fn run_worker(mut worker: Worker) {
    while let Some(msg) = worker.inbox.recv().await {
        // worker.handle_message(msg).await;
        let _ = Worker::handle_message(
            &worker.sender_to_manager,
            &mut worker.senders_to_succs,
            &mut worker.replica,
            &mut worker.curr_val,
            &mut worker.def_expr,
            &mut worker.name,
            &msg,
        )
        .await;
    }
}

pub enum LockType {
    RLock,
    WLock,
}

pub enum VarOrDef {
    Var,
    Def,
}

pub struct ServiceManager {
    // channels
    pub worker_inboxes: HashMap<String, mpsc::Sender<Message>>,
    pub sender_to_manager: mpsc::Sender<Message>,
    pub receiver_from_workers: mpsc::Receiver<Message>,
    // locks
    pub locks: HashMap<String, Option<LockType>>,
    // typing env
    pub typenv: HashMap<String, Option<typecheck::Type>>,
    pub var_or_def_env: HashMap<String, VarOrDef>,
    // dependency graph
    pub dependgraph: HashMap<String, HashSet<String>>,
}

impl ServiceManager {
    pub fn new() -> Self {
        let (sndr, rcvr) = mpsc::channel(BUFFER_SIZE);
        ServiceManager {
            worker_inboxes: HashMap::new(),
            sender_to_manager: sndr,
            receiver_from_workers: rcvr,
            locks: HashMap::new(),
            typenv: HashMap::new(),
            var_or_def_env: HashMap::new(),
            dependgraph: HashMap::new(),
        }
    }

    pub fn create_worker(
        name: &str,
        workertype: VarOrDef,
        sender_to_manager: mpsc::Sender<Message>,
        subscribers: &HashSet<String>,

        worker_inboxes: &mut HashMap<String, mpsc::Sender<Message>>,
        locks: &mut HashMap<String, Option<LockType>>,
        typenv: &mut HashMap<String, Option<typecheck::Type>>,
        var_or_def_env: &mut HashMap<String, VarOrDef>,
        dependgraph: &mut HashMap<String, HashSet<String>>,
    ) {
        let (sndr, rcvr) = mpsc::channel(BUFFER_SIZE);
        let mut subscriber_addrs = vec![];
        for n in subscribers.iter() {
            subscriber_addrs.push((worker_inboxes.get(n)).expect("Worker not exists").clone());
        }
        let worker = Worker::new(rcvr, sender_to_manager.clone(), subscriber_addrs, name);
        tokio::spawn(run_worker(worker));

        worker_inboxes.insert(name.to_string(), sndr);
        locks.insert(name.to_string(), None);
        typenv.insert(name.to_string(), None);
        var_or_def_env.insert(name.to_string(), workertype);
        dependgraph.insert(name.to_string(), subscribers.clone());
    }

    pub async fn init_var_worker(
        worker_inboxes: &mut HashMap<String, mpsc::Sender<Message>>,
        name: &str,
        var_init_val: meerast::Expr,
    ) {
        let worker_addr = worker_inboxes.get(name).unwrap();
        let msg = Message::InitVar {
            var_name: name.to_string(),
            var_expr: var_init_val,
        };
        let _ = worker_addr.send(msg).await.expect("Init val fails");
    }

    pub async fn init_def_worker(
        worker_inboxes: &HashMap<String, mpsc::Sender<Message>>,
        name: &str,
        def_init_expr: meerast::Expr,
    ) {
        let worker_addr = worker_inboxes.get(name).unwrap();
        let msg = Message::InitDef {
            def_name: name.to_string(),
            def_expr: def_init_expr,
        };
        let _ = worker_addr.send(msg).await.expect("Init def fails");
    }

    pub async fn retrieve_val(
        worker_inboxes: &HashMap<String, mpsc::Sender<Message>>,
        receiver_from_workers: &mut mpsc::Receiver<Message>,
        name: &str,
    ) -> Option<Val> {
        let worker_addr = worker_inboxes.get(name).unwrap();
        let msg = Message::RetrieveVal;
        let _ = worker_addr
            .send(msg)
            .await
            .expect("No val retreived from actor");
        match receiver_from_workers
            .recv()
            .await
            .expect("No val retrieved from worker")
        {
            Message::AppriseVal {
                worker_name,
                worker_value,
            } => worker_value,
            _ => panic!("unexpected message from worker to service manager"),
        }
    }
}

// TODO:
// create a new developer thread

// syntax abstraction,
// statically evaluates read/write set
// type check
