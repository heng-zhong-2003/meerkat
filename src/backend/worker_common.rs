use crate::backend::message::Message;
use tokio::sync::mpsc;

pub struct WorkerCommon {
    pub name: String,
    pub inbox_receiver: mpsc::Receiver<Message>,
    pub sender_to_manager: mpsc::Sender<Message>,
    pub senders_to_succs: Vec<mpsc::Sender<Message>>,
}

impl WorkerCommon {
    pub fn new(
        name: &str,
        inbox_receiver: mpsc::Receiver<Message>,
        sender_to_manager: mpsc::Sender<Message>,
    ) -> WorkerCommon {
        WorkerCommon {
            name: name.to_string(),
            inbox_receiver,
            sender_to_manager,
            senders_to_succs: Vec::new(),
        }
    }
}
