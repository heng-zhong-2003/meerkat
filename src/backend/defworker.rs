use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use tokio::sync::mpsc;

use crate::{
    backend::{
        message::{Message, PropaChange, Val},
        transaction::Txn,
        worker_common::WorkerCommon,
    },
    frontend::meerast::Expr,
};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TxnAndName {
    pub txn: Txn,
    pub name: String,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ExtendedPropaChange {
    pub propa_id: i32,
    pub propa_change: PropaChange,
    pub deps: HashSet<TxnAndName>,
}

impl Hash for ExtendedPropaChange {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.propa_id.hash(state);
    }
}

pub struct DefWorker {
    pub worker_common: WorkerCommon,
    pub value: Option<Val>,
    pub applied_txns: Vec<Txn>,
    pub prev_batch_provide: HashSet<Txn>,
    // all propa_change's to be applied
    pub propa_changes_to_apply: HashMap<TxnAndName, ExtendedPropaChange>,
    pub expr: Expr,
    pub all_inputs_ready: bool,
    pub replica: HashMap<String, Option<Val>>,
    pub transitive_deps: HashMap<String, HashSet<String>>,
    pub counter: i32,
}

impl DefWorker {
    pub fn new(
        name: &str,
        inbox_receiver: mpsc::Receiver<Message>,
        sender_to_manager: mpsc::Sender<Message>,
        expr: Expr,
        replica: HashMap<String, Option<Val>>,
        transitive_deps: HashMap<String, HashSet<String>>,
    ) -> Self {
        DefWorker {
            worker_common: WorkerCommon::new(name, inbox_receiver, sender_to_manager),
            value: None,
            applied_txns: Vec::new(),
            prev_batch_provide: HashSet::new(),
            propa_changes_to_apply: HashMap::new(),
            expr,
            all_inputs_ready: false,
            replica,
            transitive_deps,
            counter: 0,
        }
    }

    pub fn next_count(counter: &mut i32) -> i32 {
        *counter += 1;
        *counter
    }

    pub async fn handle_message(
        worker_common: &WorkerCommon,
        value: &mut Option<Val>,
        counter: &mut i32,
        transitive_deps: &HashMap<String, HashSet<String>>,
        propa_changes_to_apply: &mut HashMap<TxnAndName, ExtendedPropaChange>,
        msg: &Message,
    ) {
        match msg {
            Message::ReadDefRequest { txn } => {
                todo!()
            }
            Message::PropaMessage { propa_change } => {
                let xpropa = todo!();
            }
            Message::ManagerRetrieveRequest => todo!(),
            _ => panic!(),
        }
    }

    pub async fn run(mut def_worker: DefWorker) {
        todo!()
    }
}
