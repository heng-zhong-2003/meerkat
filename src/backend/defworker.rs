use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use tokio::sync::mpsc;

use crate::{
    backend::{
        manager::Manager,
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
                let xpropa =
                    DefWorker::annotate_propa_change(counter, propa_change, transitive_deps);
                for txn in propa_change.provide.iter() {
                    propa_changes_to_apply.insert(
                        TxnAndName {
                            txn: txn.clone(),
                            name: propa_change.name.clone(),
                        },
                        xpropa.clone(),
                    );
                }
            }
            Message::ManagerRetrieveRequest => todo!(),
            _ => panic!(),
        }
    }

    pub async fn run(mut def_worker: DefWorker) {
        while let Some(msg) = def_worker.worker_common.inbox_receiver.recv().await {
            DefWorker::handle_message(
                &def_worker.worker_common,
                &mut def_worker.value,
                &mut def_worker.counter,
                &def_worker.transitive_deps,
                &mut def_worker.propa_changes_to_apply,
                &msg,
            )
            .await;
            let valid_batch = DefWorker::search_batch(
                &def_worker.propa_changes_to_apply,
                &def_worker.applied_txns,
            );
            let (all_provides, all_requires, new_value) = DefWorker::apply_batch(
                &def_worker.expr,
                valid_batch,
                &mut def_worker.value,
                &mut def_worker.applied_txns,
                &mut def_worker.prev_batch_provide,
                &mut def_worker.propa_changes_to_apply,
                &mut def_worker.replica,
            );
        }
    }

    pub fn annotate_propa_change(
        counter: &mut i32,
        propa_change: &PropaChange,
        transitive_deps: &HashMap<String, HashSet<String>>,
    ) -> ExtendedPropaChange {
        let mut deps: HashSet<TxnAndName> = HashSet::new();
        for txn in propa_change.provide.iter() {
            for write in txn.writes.iter() {
                let var_name = write.name.clone();
                let mut inputs: Vec<String> = vec![];
                for (i, dep_vars) in transitive_deps.iter() {
                    match dep_vars.get(&var_name) {
                        Some(_) => {
                            inputs.push(i.clone());
                        }
                        None => {}
                    }
                }
                for i_name in inputs.into_iter() {
                    let txn_name = TxnAndName {
                        txn: txn.clone(),
                        name: i_name,
                    };
                    deps.insert(txn_name);
                }
            }
        }
        for txn in propa_change.require.iter() {
            for write in txn.writes.iter() {
                let var_name = write.name.clone();
                let mut inputs: Vec<String> = vec![];
                for (i, dep_vars) in transitive_deps.iter() {
                    match dep_vars.get(&var_name) {
                        Some(_) => {
                            inputs.push(i.clone());
                        }
                        None => {}
                    }
                }
                for i_name in inputs.into_iter() {
                    let txn_name = TxnAndName {
                        txn: txn.clone(),
                        name: i_name,
                    };
                    deps.insert(txn_name);
                }
            }
        }
        ExtendedPropaChange {
            propa_id: DefWorker::next_count(counter),
            propa_change: propa_change.clone(),
            deps,
        }
    }

    fn dfs(
        curr_node: &TxnAndName,
        visited: &mut HashSet<TxnAndName>,
        batch_acc: &mut HashSet<ExtendedPropaChange>,
        applied_txns: &HashSet<Txn>,
        graph: &HashMap<TxnAndName, ExtendedPropaChange>,
    ) -> bool {
        if visited.get(curr_node) != None || applied_txns.get(&curr_node.txn) != None {
            return true;
        } else {
            match graph.get(curr_node) {
                Some(xpropa) => {
                    visited.insert(curr_node.clone());
                    batch_acc.insert(xpropa.clone());
                    for succ in xpropa.deps.iter() {
                        if !DefWorker::dfs(succ, visited, batch_acc, applied_txns, graph) {
                            return false;
                        }
                    }
                    return true;
                }
                None => {
                    return false;
                }
            }
        }
    }

    fn search_batch(
        propa_changes_to_apply: &HashMap<TxnAndName, ExtendedPropaChange>,
        applied_txns: &Vec<Txn>,
    ) -> HashSet<ExtendedPropaChange> {
        let applied_txns_set: HashSet<Txn> = applied_txns.iter().cloned().collect();
        let mut visited: HashSet<TxnAndName> = HashSet::new();
        let mut batch_acc: HashSet<ExtendedPropaChange> = HashSet::new();

        for (node, _) in propa_changes_to_apply.iter() {
            if DefWorker::dfs(
                node,
                &mut visited,
                &mut batch_acc,
                &applied_txns_set,
                &propa_changes_to_apply,
            ) {
                return batch_acc;
            } else {
                visited = HashSet::new();
                batch_acc = HashSet::new();
            }
        }
        batch_acc
    }

    fn apply_batch(
        def_expr: &Expr,
        batch: HashSet<ExtendedPropaChange>,
        value: &mut Option<Val>,
        applied_txns: &mut Vec<Txn>,
        prev_batch_provide: &mut HashSet<Txn>,
        propa_changes_to_apply: &mut HashMap<TxnAndName, ExtendedPropaChange>,
        replica: &mut HashMap<String, Option<Val>>,
        // return: (all_provides, all_requires, new_value)
    ) -> (HashSet<Txn>, HashSet<Txn>, Option<Val>) {
        let mut all_provides: HashSet<Txn> = HashSet::new();
        let mut all_requires: HashSet<Txn> = prev_batch_provide.clone();
        let mut latest_change: HashMap<String, i32> = HashMap::new();
        for change in batch.iter() {
            let change_txns_to_apply = &change.propa_change.provide;
            all_provides = all_provides.union(change_txns_to_apply).cloned().collect();
            all_requires = all_requires
                .union(&change.propa_change.require)
                .cloned()
                .collect();
            for txn in change_txns_to_apply.iter() {
                propa_changes_to_apply.remove(&TxnAndName {
                    txn: txn.clone(),
                    name: change.propa_change.name.clone(),
                });
            }
            if let Some(id) = latest_change.get(&change.propa_change.name) {
                if change.propa_id < *id {
                    continue;
                }
            }
            replica.insert(
                change.propa_change.name.clone(),
                Some(change.propa_change.new_val.clone()),
            );
            latest_change.insert(change.propa_change.name.clone(), change.propa_id);
        }
        *value = Manager::evaluate_txn_expr(def_expr, replica);
        for txn in all_provides.iter() {
            applied_txns.push(txn.clone());
        }
        *prev_batch_provide = all_provides.clone();
        (all_provides, all_requires, value.clone())
    }

    // fn evaluate_val_of_def(expr: &Expr, replica: HashMap<String, Option<Val>>) -> Option<Val>
    // use Manager::evaluate_txn_expr for this, refactor in the future
}
