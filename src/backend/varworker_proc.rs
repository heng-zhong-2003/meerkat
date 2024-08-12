use crate::backend::srvmanager_proc;
use crate::backend::worker;
use crate::backend::transaction;
use crate::backend::message;
use tokio::sync::mpsc;
use std::collections::{HashSet, HashMap};

// pub struct VarWorker {
//     pub worker: worker::Worker,
//     pub applied_txns: HashSet<transaction::Txn>,
//     pub provides: HashSet<transaction::Txn>,
//     pub requires: HashSet<transaction::Txn>,
// }

// impl VarWorker {
//     pub fn new(
//         inbox: mpsc::Receiver<message::Message>,
//         sender_to_manager: mpsc::Sender<message::Message>,
//         name: &str,
//         // replica: HashMap<String, Option<message::Val>>,
//         // def_expr: Option<meerast::Expr>,
//     ) -> VarWorker {
//         VarWorker {
//             worker: worker::Worker {
//                 inbox,
//                 sender_to_manager,
//                 senders_to_succs: Vec::new(),
//                 replica: HashMap::new(),
//                 curr_val: None,
//                 def_expr: None,
//                 name: name.to_string(),
//             },
//             applied_txns: HashSet::new(),
//             provides: HashSet::new(),
//             requires: HashSet::new(),
//         }
//     }

//     pub async fn handle_message(
//         sender_to_manager: &mpsc::Sender<message::Message>,
//         senders_to_succs: &mut Vec<mpsc::Sender<message::Message>>,
//         // replica: &mut HashMap<String, Option<message::Val>>,
//         curr_val: &mut Option<message::Val>,
//         // def_expr: &mut Option<meerast::Expr>,
//         name: &mut String,
//         msg: &message::Message,
//     ) {
//         match msg {

//         }
//     }
// }

pub async fn var_proc() {
    loop {
        todo!()
    }
}
