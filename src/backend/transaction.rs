use core::fmt;
use std::hash::{Hash, Hasher};
use tokio::time::Instant;

use crate::{backend::message::Val, frontend::meerast::Expr};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Hash)]
pub struct TxnId {
    pub time: Instant,
    // address or uuid to break tie
}

impl TxnId {
    pub fn new() -> TxnId {
        TxnId {
            time: Instant::now(),
        }
    }
}

// a single update to state var
#[derive(Clone, Debug)]
pub struct WriteToName {
    pub name: String,
    pub expr: Expr,
}

// (txid, writes)
// writes := a list of updates to state vars
// Clone, PartialEq, Eq, Hash, Debug
#[derive(Clone)]
pub struct Txn {
    pub id: TxnId,
    pub writes: Vec<WriteToName>,
}

impl PartialEq for Txn {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Txn {}

impl Hash for Txn {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl fmt::Debug for Txn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Txn").finish()
    }
}
