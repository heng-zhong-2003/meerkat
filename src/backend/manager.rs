use std::{
    collections::{HashMap, HashSet},
    iter,
};

use tokio::{self, sync::mpsc};

use crate::{
    backend::{
        defworker::DefWorker,
        message::{Lock, Message, Val},
        transaction::Txn,
        varworker::VarWorker,
    },
    frontend::{
        meerast::{Binop, Expr, SglStmt, Stmt, Uop},
        typecheck::Type,
    },
};

pub const BUFFER_SIZE: usize = 1024;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum VarOrDef {
    Var,
    Def,
}

pub struct Manager {
    pub workers_inboxes_senders: HashMap<String, mpsc::Sender<Message>>,
    pub sender_to_manager: mpsc::Sender<Message>,
    pub receiver_from_workers: mpsc::Receiver<Message>,
    pub locks: HashMap<String, Option<Lock>>,
    pub type_env: HashMap<String, Option<Type>>,
    pub var_or_def_env: HashMap<String, VarOrDef>,
    pub dependency_graph: HashMap<String, HashSet<String>>,
}

impl Manager {
    pub fn new() -> Self {
        let (sndr, rcvr): (mpsc::Sender<Message>, mpsc::Receiver<Message>) =
            mpsc::channel(BUFFER_SIZE);
        Manager {
            workers_inboxes_senders: HashMap::new(),
            sender_to_manager: sndr,
            receiver_from_workers: rcvr,
            locks: HashMap::new(),
            type_env: HashMap::new(),
            var_or_def_env: HashMap::new(),
            dependency_graph: HashMap::new(),
        }
    }

    pub async fn create_var_worker(
        name: &str,
        sender_to_manager: mpsc::Sender<Message>,
        workers_inboxes_senders: &mut HashMap<String, mpsc::Sender<Message>>,
        type_env: &mut HashMap<String, Option<Type>>,
        var_or_def_env: &mut HashMap<String, VarOrDef>,
        // dependency_graph: &mut HashMap<String, HashSet<String>>,
    ) {
        let (sndr, rcvr): (mpsc::Sender<Message>, mpsc::Receiver<Message>) =
            mpsc::channel(BUFFER_SIZE);
        let var_worker = VarWorker::new(name, rcvr, sender_to_manager);
        workers_inboxes_senders.insert(name.to_string(), sndr);
        type_env.insert(name.to_string(), None);
        var_or_def_env.insert(name.to_string(), VarOrDef::Var);
        tokio::spawn(VarWorker::run(var_worker));
    }

    pub async fn create_def_worker(
        name: &str,
        sender_to_manager: mpsc::Sender<Message>,
        expr: Expr,
        replica: HashMap<String, Option<Val>>,
        transitive_deps: HashMap<String, HashSet<String>>,
        workers_inboxes_senders: &mut HashMap<String, mpsc::Sender<Message>>,
        type_env: &mut HashMap<String, Option<Type>>,
        var_or_def_env: &mut HashMap<String, VarOrDef>,
    ) {
        let (sndr, rcvr): (mpsc::Sender<Message>, mpsc::Receiver<Message>) =
            mpsc::channel(BUFFER_SIZE);
        let def_worker = DefWorker::new(
            name,
            rcvr,
            sender_to_manager.clone(),
            expr,
            replica.clone(),
            transitive_deps,
        );
        workers_inboxes_senders.insert(name.to_string(), sndr.clone());
        for (dep_name, _) in replica.iter() {
            let sender_to_pred = workers_inboxes_senders.get(dep_name).unwrap().clone();
            let _ = sender_to_pred
                .send(Message::SubscriberRequest {
                    subscriber_name: name.to_string(),
                    sender: sndr.clone(),
                })
                .await;
        }
        type_env.insert(name.to_string(), None);
        var_or_def_env.insert(name.to_string(), VarOrDef::Def);
        tokio::spawn(DefWorker::run(def_worker));
    }

    pub async fn handle_transaction(
        txn: &Txn,
        var_or_def_env: &HashMap<String, VarOrDef>,
        workers_inboxes_senders: &mut HashMap<String, mpsc::Sender<Message>>,
        receiver_from_workers: &mut mpsc::Receiver<Message>,
    ) {
        // println!("handle transaction {:?}", txn);
        let mut cnt = 0;
        // Require set of the transaction
        let mut requires_for_txn: HashSet<Txn> = HashSet::new();
        let mut names_to_values: HashMap<String, Option<Val>> = HashMap::new();
        for assign in txn.writes.iter() {
            let names_in_assign_expr = Manager::get_names_in_expr(&assign.expr);
            for depended_name in names_in_assign_expr.iter() {
                cnt += 1;
                names_to_values.insert(depended_name.clone(), None);
                Manager::send_read_request_msg_to(
                    depended_name,
                    txn,
                    var_or_def_env,
                    workers_inboxes_senders,
                )
                .await;
            }
        }
        if cnt > 0 {
            while let Some(msg_back) = receiver_from_workers.recv().await {
                match msg_back {
                    Message::ReadVarResult {
                        txn: txn_back,
                        name,
                        result,
                        result_provide,
                    } => {
                        if txn_back == *txn {
                            cnt -= 1;
                            names_to_values.insert(name.clone(), result);
                            requires_for_txn =
                                requires_for_txn.union(&result_provide).cloned().collect();
                        }
                        if cnt == 0 {
                            break;
                        }
                    }
                    Message::ReadDefResult {
                        txn: txn_back,
                        name,
                        result,
                        result_provide,
                    } => {
                        if txn_back == *txn {
                            cnt -= 1;
                            names_to_values.insert(name, result);
                        }
                        if cnt == 0 {
                            break;
                        }
                    }
                    _ => panic!(),
                }
            }
        }
        if cnt == 0 {
            for write in txn.writes.iter() {
                let opt_new_val = Manager::evaluate_txn_expr(&write.expr, &names_to_values);
                let new_val = match opt_new_val {
                    Some(v) => v,
                    None => {
                        continue;
                    }
                };
                let msg_write_request = Message::WriteVarRequest {
                    txn: txn.clone(),
                    write_val: new_val,
                    requires: requires_for_txn.clone(),
                };
                let var_inbox_sender = workers_inboxes_senders.get(&write.name).unwrap();
                let _ = var_inbox_sender.send(msg_write_request).await;
            }
            return;
        }
    }

    async fn send_read_request_msg_to(
        name: &str,
        txn: &Txn,
        var_or_def_env: &HashMap<String, VarOrDef>,
        workers_inboxes_senders: &HashMap<String, mpsc::Sender<Message>>,
    ) {
        let var_or_def = var_or_def_env.get(name).unwrap();
        let read_request_msg = match var_or_def {
            VarOrDef::Var => Message::ReadVarRequest { txn: txn.clone() },
            VarOrDef::Def => Message::ReadDefRequest {
                txn: txn.clone(),
                require: HashSet::new(), // TODO
            },
        };
        let worker_inbox_sender = workers_inboxes_senders.get(name).unwrap();
        let _ = worker_inbox_sender.send(read_request_msg).await.unwrap();
    }

    pub async fn retrieve_val_of(
        name: &str,
        workers_inboxes_senders: &HashMap<String, mpsc::Sender<Message>>,
        receiver_from_workers: &mut mpsc::Receiver<Message>,
    ) -> Option<Val> {
        let retrieve_request_msg = Message::ManagerRetrieveRequest;
        let worker_inbox_sender = workers_inboxes_senders.get(name).unwrap();
        println!("retrieve val of {}", name);
        let _ = match worker_inbox_sender.send(retrieve_request_msg).await {
            Ok(_) => {}
            Err(em) => println!("retrieve val send error: {}", em),
        };
        if let Some(msg) = receiver_from_workers.recv().await {
            match msg {
                Message::ManagerRetrieveResult {
                    name: result_name,
                    result,
                } => {
                    if result_name != name {
                        panic!()
                    }
                    return result;
                }
                _ => panic!(),
            }
        }
        panic!()
    }

    pub fn evaluate_txn_expr(
        expr: &Expr,
        names_to_values: &HashMap<String, Option<Val>>,
    ) -> Option<Val> {
        match expr {
            Expr::IdExpr { ident } => {
                return match names_to_values.get(ident) {
                    Some(Some(v)) => Some(v.clone()),
                    _ => None,
                };
            }
            Expr::IntConst { val } => Some(Val::Int(val.clone())),
            Expr::BoolConst { val } => Some(Val::Bool(val.clone())),
            Expr::Action { stmt: _ } => Some(Val::Action(expr.clone())),
            Expr::Member {
                srv_name: _,
                member: _,
            } => panic!(),
            Expr::Apply { fun, args } => {
                let opt_substed_body =
                    Manager::subst_pars_of_apply_for_args(fun, args, names_to_values);
                let substed_body = match opt_substed_body {
                    Some(bd) => bd,
                    None => return None,
                };
                Manager::evaluate_txn_expr(&substed_body, names_to_values)
            }
            Expr::BopExpr { opd1, opd2, bop } => {
                let opt_val1 = Manager::evaluate_txn_expr(opd1, names_to_values);
                let opt_val2 = Manager::evaluate_txn_expr(opd2, names_to_values);
                let (val1, val2) = match (opt_val1, opt_val2) {
                    (Some(v1), Some(v2)) => (v1, v2),
                    _ => {
                        return None;
                    }
                };
                Some(Manager::evaluate_binop_vals(&val1, &val2, bop))
            }
            Expr::UopExpr { opd, uop } => {
                let opt_val = Manager::evaluate_txn_expr(opd, names_to_values);
                let val = match opt_val {
                    Some(v) => v,
                    None => return None,
                };
                Some(Manager::evaluate_uop_vals(&val, uop))
            }
            Expr::IfExpr { cond, then, elze } => {
                let opt_cond = Manager::evaluate_txn_expr(cond, names_to_values);
                let evaled_cond = match opt_cond {
                    Some(Val::Bool(b)) => b,
                    _ => {
                        return None;
                    }
                };
                if evaled_cond {
                    Manager::evaluate_txn_expr(then, names_to_values)
                } else {
                    Manager::evaluate_txn_expr(elze, names_to_values)
                }
            }
            Expr::Lambda { pars: _, body: _ } => Some(Val::Lambda(expr.clone())),
        }
    }

    fn subst_pars_of_apply_for_args(
        fun: &Expr,
        args: &Vec<Expr>,
        names_to_values: &HashMap<String, Option<Val>>,
    ) -> Option<Expr> {
        let fun_val = Manager::evaluate_txn_expr(fun, names_to_values);
        let fun_val = match fun_val {
            Some(v) => v,
            None => {
                return None;
            }
        };
        let fun = &match fun_val {
            Val::Int(_) => panic!(),
            Val::Bool(_) => panic!(),
            Val::Action(_) => panic!(),
            Val::Lambda(e) => e,
        };
        let pars = match fun {
            Expr::Lambda { pars: ps, body: _ } => ps,
            Expr::IdExpr { ident } => {
                let fun_lambda = match names_to_values.get(ident) {
                    Some(val) => match val {
                        Some(Val::Lambda(lam)) => lam,
                        _ => panic!(),
                    },
                    None => panic!(),
                };
                match fun_lambda {
                    Expr::Lambda { pars: ps, body: _ } => ps,
                    _ => panic!(),
                }
            }
            _ => panic!(),
        };
        let body = match fun {
            Expr::Lambda { pars: _, body: bd } => bd,
            Expr::IdExpr { ident } => {
                let fun_lambda = match names_to_values.get(ident) {
                    Some(val) => match val {
                        Some(Val::Lambda(lam)) => lam,
                        _ => panic!(),
                    },
                    None => panic!(),
                };
                match fun_lambda {
                    Expr::Lambda { pars: _, body: bd } => bd,
                    _ => panic!(),
                }
            }
            _ => panic!(),
        };
        let mut par_arg_map: HashMap<String, Expr> = HashMap::new();
        for (par, arg) in iter::zip(pars.iter(), args.iter()) {
            let par_ident = match par {
                Expr::IdExpr { ident } => ident.clone(),
                _ => panic!(),
            };
            par_arg_map.insert(par_ident, arg.clone());
        }
        let mut substed_expr = *body.clone();
        Manager::subst(&mut substed_expr, &par_arg_map);
        Some(substed_expr)
    }

    fn subst(expr: &mut Expr, ident_expr_map: &HashMap<String, Expr>) {
        match expr {
            Expr::IdExpr { ident } => {
                let ent = ident_expr_map.get(ident);
                match ent {
                    Some(e) => {
                        *expr = e.clone();
                    }
                    None => {}
                }
            }
            Expr::IntConst { val } => {}
            Expr::BoolConst { val } => {}
            Expr::Action { stmt } => {
                let sgls = match stmt {
                    Stmt::Stmt { sgl_stmts } => sgl_stmts,
                };
                for sgl_stmt in sgls.iter_mut() {
                    match sgl_stmt {
                        SglStmt::Do { act } => {
                            Manager::subst(act, ident_expr_map);
                        }
                        SglStmt::Ass { dst: _, src } => {
                            Manager::subst(src, ident_expr_map);
                        }
                    }
                }
            }
            Expr::Member {
                srv_name: _,
                member: _,
            } => panic!(),
            Expr::Apply { fun, args } => {
                Manager::subst(fun, ident_expr_map);
                for arg in args.iter_mut() {
                    Manager::subst(arg, ident_expr_map);
                }
            }
            Expr::BopExpr { opd1, opd2, bop: _ } => {
                Manager::subst(opd1, ident_expr_map);
                Manager::subst(opd2, ident_expr_map);
            }
            Expr::UopExpr { opd, uop: _ } => {
                Manager::subst(opd, ident_expr_map);
            }
            Expr::IfExpr { cond, then, elze } => {
                Manager::subst(cond, ident_expr_map);
                Manager::subst(then, ident_expr_map);
                Manager::subst(elze, ident_expr_map);
            }
            Expr::Lambda { pars, body } => {
                let par_names: HashSet<String> = pars
                    .iter()
                    .map(|p| match p {
                        Expr::IdExpr { ident } => ident.clone(),
                        _ => panic!(),
                    })
                    .collect();
                let mut body_map: HashMap<String, Expr> = HashMap::new();
                for (ident, arg_expr) in ident_expr_map.iter() {
                    if !par_names.contains(ident) {
                        body_map.insert(ident.clone(), arg_expr.clone());
                    }
                }
                Manager::subst(body, &body_map);
            }
        }
    }

    pub fn subst_idents_in_expr_for_vals(
        expr: &Expr,
        names_to_values: &HashMap<String, Val>,
    ) -> Expr {
        let mut subst_map: HashMap<String, Val> = HashMap::new();
        for (n, v) in names_to_values.iter() {
            subst_map.insert(n.clone(), v.clone());
        }
        match expr {
            Expr::IdExpr { ident } => {
                let val_for_ident = subst_map.get(ident).unwrap();
                match val_for_ident {
                    Val::Int(i) => Expr::IntConst { val: *i },
                    Val::Bool(b) => Expr::BoolConst { val: *b },
                    Val::Action(act) => act.clone(),
                    Val::Lambda(fun) => fun.clone(),
                }
            }
            Expr::IntConst { val } => Expr::IntConst { val: *val },
            Expr::BoolConst { val } => Expr::BoolConst { val: *val },
            Expr::Action { stmt: _ } => expr.clone(),
            Expr::Member {
                srv_name: _,
                member: _,
            } => panic!(),
            Expr::Apply { fun, args } => {
                let substed_fun = Manager::subst_idents_in_expr_for_vals(fun, names_to_values);
                let substed_args: Vec<Expr> = args
                    .iter()
                    .map(|arg| Manager::subst_idents_in_expr_for_vals(arg, names_to_values))
                    .collect();
                Expr::Apply {
                    fun: Box::new(substed_fun),
                    args: substed_args,
                }
            }
            Expr::BopExpr { opd1, opd2, bop } => {
                let substed_opd1 = Manager::subst_idents_in_expr_for_vals(opd1, names_to_values);
                let substed_opd2 = Manager::subst_idents_in_expr_for_vals(opd2, names_to_values);
                Expr::BopExpr {
                    opd1: Box::new(substed_opd1),
                    opd2: Box::new(substed_opd2),
                    bop: bop.clone(),
                }
            }
            Expr::UopExpr { opd, uop } => {
                let substed_opd = Manager::subst_idents_in_expr_for_vals(opd, names_to_values);
                Expr::UopExpr {
                    opd: Box::new(substed_opd),
                    uop: uop.clone(),
                }
            }
            Expr::IfExpr { cond, then, elze } => {
                let substed_cond = Manager::subst_idents_in_expr_for_vals(cond, names_to_values);
                let substed_then = Manager::subst_idents_in_expr_for_vals(then, names_to_values);
                let substed_elze = Manager::subst_idents_in_expr_for_vals(elze, names_to_values);
                Expr::IfExpr {
                    cond: Box::new(substed_cond),
                    then: Box::new(substed_then),
                    elze: Box::new(substed_elze),
                }
            }
            Expr::Lambda { pars: _, body: _ } => expr.clone(),
        }
    }

    fn evaluate_binop_vals(val1: &Val, val2: &Val, bop: &Binop) -> Val {
        match (val1, val2) {
            (Val::Int(i), Val::Int(j)) => match bop {
                Binop::Add => Val::Int(i + j),
                Binop::Sub => Val::Int(i - j),
                Binop::Mul => Val::Int(i * j),
                Binop::Div => Val::Int(i / j),
                Binop::Eq => Val::Bool(i == j),
                Binop::Lt => Val::Bool(i < j),
                Binop::Gt => Val::Bool(i > j),
                Binop::And | Binop::Or => panic!(),
            },
            (Val::Bool(b1), Val::Bool(b2)) => match bop {
                Binop::Add
                | Binop::Sub
                | Binop::Mul
                | Binop::Div
                | Binop::Eq
                | Binop::Lt
                | Binop::Gt => panic!(),
                Binop::And => Val::Bool(*b1 && *b2),
                Binop::Or => Val::Bool(*b1 || *b2),
            },
            _ => panic!(),
        }
    }

    fn evaluate_uop_vals(val: &Val, uop: &Uop) -> Val {
        match val {
            Val::Int(i) => match uop {
                Uop::Neg => Val::Int(-i),
                Uop::Not => panic!(),
            },
            Val::Bool(b) => match uop {
                Uop::Neg => panic!(),
                Uop::Not => Val::Bool(!b),
            },
            _ => panic!(),
        }
    }

    pub fn get_names_in_expr(expr: &Expr) -> HashSet<String> {
        let mut result: HashSet<String> = HashSet::new();
        match expr {
            Expr::IdExpr { ident } => {
                result.insert(ident.clone());
            }
            Expr::IntConst { val: _ } | Expr::BoolConst { val: _ } => {}
            Expr::Action { stmt: _ } => {}
            Expr::Member {
                srv_name: _,
                member: _,
            } => panic!(),
            Expr::Apply { fun, args } => {
                let names_in_fun = Manager::get_names_in_expr(fun);
                for nm in names_in_fun.into_iter() {
                    result.insert(nm);
                }
                for arg in args.iter() {
                    let names_in_arg = Manager::get_names_in_expr(arg);
                    for nm in names_in_arg.into_iter() {
                        result.insert(nm);
                    }
                }
            }
            Expr::BopExpr { opd1, opd2, bop: _ } => {
                let names_in_opd1 = Manager::get_names_in_expr(opd1);
                let names_in_opd2 = Manager::get_names_in_expr(opd2);
                for nm in names_in_opd1.into_iter() {
                    result.insert(nm);
                }
                for nm in names_in_opd2.into_iter() {
                    result.insert(nm);
                }
            }
            Expr::UopExpr { opd, uop: _ } => {
                let names_in_opd = Manager::get_names_in_expr(opd);
                for nm in names_in_opd.into_iter() {
                    result.insert(nm);
                }
            }
            Expr::IfExpr { cond, then, elze } => {
                let names_in_cond = Manager::get_names_in_expr(cond);
                let names_in_then = Manager::get_names_in_expr(then);
                let names_in_elze = Manager::get_names_in_expr(elze);
                for nm in names_in_cond.into_iter() {
                    result.insert(nm);
                }
                for nm in names_in_then.into_iter() {
                    result.insert(nm);
                }
                for nm in names_in_elze.into_iter() {
                    result.insert(nm);
                }
            }
            Expr::Lambda { pars: _, body: _ } => {}
        }
        result
    }
}
