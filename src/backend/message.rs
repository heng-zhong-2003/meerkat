use std::collections::HashSet;
use tokio::sync::mpsc;

use crate::backend::transaction::Txn;
use crate::frontend::meerast::Expr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    Int(i32),
    Bool(bool),
    Action(Expr), /* Expr have to be Action */
    Lambda(Expr), /* Expr have to be Lambda */
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PropaChange {
    pub name: String,
    pub new_val: Val,
    pub provide: HashSet<Txn>,
    pub require: HashSet<Txn>,
}

#[derive(Debug, Clone)]
pub enum LockKind {
    WLock,
    RLock,
}

pub struct Lock {
    pub txn: Txn,
    pub lock_kind: LockKind,
}

#[derive(Debug, Clone)]
pub enum Message {
    // manager -> var worker
    ReadVarRequest {
        txn: Txn,
    },
    // manager -> def worker
    ReadDefRequest {
        txn: Txn,
    },
    // var worker -> manager
    ReadVarResult {
        txn: Txn,
        name: String,
        result: Option<Val>,
        result_provide: HashSet<Txn>,
    },
    // def worker -> manager
    ReadDefResult {
        txn: Txn,
        name: String,
        result: Option<Val>,
    },
    // manager -> var worker
    WriteVarRequest {
        txn: Txn,
        write_val: Val,
        requires: HashSet<Txn>,
    },
    // var worker -> def worker (succs)
    PropaMessage {
        propa_change: PropaChange,
    },

    // manager -> [var/def] worker
    ManagerRetrieveRequest,
    // [var/def] worker -> manager
    ManagerRetrieveResult {
        name: String,
        result: Option<Val>,
    },

    // For creating def workers
    SubscriberRequest {
        subscriber_name: String,
        sender: mpsc::Sender<Message>,
    },
    SubscriberGrant {},
}
