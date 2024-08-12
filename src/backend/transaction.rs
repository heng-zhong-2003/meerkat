use tokio::time::Instant;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct TxnId {
    pub time: Instant,
    // address or uuid to break tie
}

impl TxnId {
    pub fn new() -> TxnId {
       TxnId{time: Instant::now(),}
    }
}

// a single update to state var 
pub struct WriteToName {
    pub name: String,
    pub expr: i32,
}

// (txid, writes)
// writes := a list of updates to state vars
pub struct Txn {
    pub id: TxnId, 
    pub writes: Vec<WriteToName>,
}

