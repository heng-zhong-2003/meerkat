use std::collections::HashSet;
use tokio::sync::mpsc;

use crate::backend::{
    message::{Message, PropaChange, Val},
    transaction::Txn,
    worker_common::WorkerCommon,
};

pub struct VarWorker {
    pub worker_common: WorkerCommon,
    pub value: Option<Val>,
    pub applied_txns: Vec<Txn>,
    pub next_requires: HashSet<Txn>,
}

impl VarWorker {
    pub fn new(
        name: &str,
        inbox_receiver: mpsc::Receiver<Message>,
        sender_to_manager: mpsc::Sender<Message>,
    ) -> VarWorker {
        VarWorker {
            worker_common: WorkerCommon::new(name, inbox_receiver, sender_to_manager),
            value: None,
            applied_txns: Vec::new(),
            next_requires: HashSet::new(),
        }
    }

    // Arguments when called (considered as a method):
    // worker_common:  &mut self.worker_common
    // value:          &mut self.value
    // applied_txns:   &mut self.applied_txns
    // next_requires:  &mut self.next_requires
    // msg:            msg
    async fn handle_message(
        worker_common: &mut WorkerCommon,
        value: &mut Option<Val>,
        applied_txns: &mut Vec<Txn>,
        next_requires: &mut HashSet<Txn>,
        msg: &Message,
    ) {
        match msg {
            Message::ReadVarRequest { txn } => {
                let latest_txn = HashSet::from([applied_txns[applied_txns.len() - 1].clone()]);
                let msg_back = Message::ReadVarResult {
                    txn: txn.clone(),
                    name: worker_common.name.clone(),
                    result: value.clone(),
                    result_provide: latest_txn,
                };
                let _ = worker_common
                    .sender_to_manager
                    .send(msg_back)
                    .await
                    .unwrap();
                next_requires.insert(txn.clone());
            }
            Message::WriteVarRequest {
                txn,
                write_val,
                requires,
            } => {
                *value = Some(write_val.clone());
                for r in requires.iter() {
                    next_requires.insert(r.clone());
                }
                let msg_propa = Message::PropaMessage {
                    propa_change: PropaChange {
                        name: worker_common.name.clone(),
                        new_val: value.clone().unwrap(),
                        provide: HashSet::from([txn.clone()]),
                        require: requires.clone(),
                    },
                };
            }
            _ => panic!(),
        }
    }

    // Arguments when called (considered as a method):
    // var_worker:     mut self
    pub async fn run(mut var_worker: VarWorker) {
        todo!()
    }
}
