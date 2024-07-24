use crate::frontend::meerast;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    Int(i32),
    Bool(bool),
    Action(meerast::Expr), /* Expr have to be Action */
    Lambda(meerast::Expr), /* Expr have to be Lambda */
}

#[derive(Debug, Clone)]
pub enum Message {
    /* Manager to worker messages */
    InitVar {
        var_name: String,
        value: meerast::Expr,
    },
    InitDef {
        def_name: String,
        value: meerast::Expr,
    },
    AddSenderToSucc {
        sender: mpsc::Sender<Message>,
    },
    RetrieveVal,
    /* Worker to manager messages */
    AppriseVal {
        worker_name: String,
        worker_value: Val,
    },
    /* Inter-worker messages */
    PredUpdatedTo {
        pred_name: String,
        pred_value: Val,
    },
}
