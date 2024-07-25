use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

use crate::{
    backend::message,
    frontend::meerast::{self, Expr},
};
use tokio::sync::mpsc;

pub struct Worker {
    pub inbox: mpsc::Receiver<message::Message>,
    pub sender_to_manager: mpsc::Sender<message::Message>,
    pub senders_to_succs: Vec<mpsc::Sender<message::Message>>,
    pub replica: HashMap<String, message::Val>,
    pub curr_val: Option<message::Val>,
    pub def_expr: Option<meerast::Expr>, /* Is `Some` only for def's */
    pub name: String,
}

impl Worker {
    pub fn new(
        inbox: mpsc::Receiver<message::Message>,
        sender_to_manager: mpsc::Sender<message::Message>,
        senders_to_succs: Vec<mpsc::Sender<message::Message>>,
        name: &str,
    ) -> Worker {
        Worker {
            inbox,
            sender_to_manager,
            senders_to_succs,
            replica: HashMap::new(),
            curr_val: None,
            def_expr: None,
            name: name.to_string(),
        }
    }

    pub async fn handle_message(
        sender_to_manager: &mpsc::Sender<message::Message>,
        senders_to_succs: &mut Vec<mpsc::Sender<message::Message>>,
        replica: &mut HashMap<String, message::Val>,
        curr_val: &mut Option<message::Val>,
        def_expr: &mut Option<meerast::Expr>,
        name: &mut String,
        msg: &message::Message,
    ) {
        match msg {
            message::Message::InitVar { var_name, var_expr } => {
                *name = var_name.clone();
                *curr_val = Some(Worker::compute_val(var_expr, replica));
            }
            message::Message::InitDef {
                def_name,
                def_expr: def_val,
            } => {
                *name = def_name.clone();
                *def_expr = Some(def_val.clone());
                *curr_val = Some(Worker::compute_val(def_val, replica));
            }
            message::Message::AddSenderToSucc { sender } => {
                senders_to_succs.push(sender.clone());
                let _ = sender
                    .send(message::Message::PredUpdatedTo {
                        pred_name: name.clone(),
                        pred_value: curr_val.clone(),
                    })
                    .await;
            }
            message::Message::RetrieveVal => {
                let _ = sender_to_manager
                    .send(message::Message::AppriseVal {
                        worker_name: name.clone(),
                        worker_value: curr_val.clone(),
                    })
                    .await;
            }
            message::Message::AppriseVal {
                worker_name: _,
                worker_value: _,
            } => {
                panic!("worker should not receive `AppriseVal` message");
            }
            message::Message::PredUpdatedTo {
                pred_name,
                pred_value,
            } => {
                if let Some(pred_value) = pred_value {
                    replica.insert(pred_name.clone(), pred_value.clone());
                    /* Re-evaluate curr_val */
                    *curr_val = Some(Worker::compute_val(
                        match def_expr {
                            Some(e) => e,
                            None => panic!(),
                        },
                        replica,
                    ));
                }
            }
        }
    }

    pub fn compute_val(
        expr: &meerast::Expr,
        replica: &HashMap<String, message::Val>,
    ) -> message::Val {
        match expr {
            meerast::Expr::IdExpr { ident } => replica.get(ident).expect("").clone(),
            meerast::Expr::IntConst { val } => message::Val::Int(val.clone()),
            meerast::Expr::BoolConst { val } => message::Val::Bool(val.clone()),
            meerast::Expr::Action { stmt: _ } => message::Val::Action(expr.clone()),
            meerast::Expr::Member {
                srv_name: _,
                member: _,
            } => panic!(),
            meerast::Expr::Apply { fun, args } => {
                let substed_fun_body = Worker::subst_pars_with_args(fun, args);
                Worker::compute_val(&substed_fun_body, replica)
            }
            meerast::Expr::BopExpr { opd1, opd2, bop } => match bop {
                meerast::Binop::Add => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Int(opd1_val + opd2_val)
                }
                meerast::Binop::Sub => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Int(opd1_val - opd2_val)
                }
                meerast::Binop::Mul => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Int(opd1_val * opd2_val)
                }
                meerast::Binop::Div => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Int(opd1_val / opd2_val)
                }
                meerast::Binop::Eq => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Bool(opd1_val == opd2_val)
                }
                meerast::Binop::Lt => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Bool(opd1_val < opd2_val)
                }
                meerast::Binop::Gt => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Bool(opd1_val > opd2_val)
                }
                meerast::Binop::And => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Bool(b) => b,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Bool(b) => b,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Bool(opd1_val && opd2_val)
                }
                meerast::Binop::Or => {
                    let opd1_val = match Worker::compute_val(opd1, replica) {
                        message::Val::Bool(b) => b,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    let opd2_val = match Worker::compute_val(opd2, replica) {
                        message::Val::Bool(b) => b,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Bool(opd1_val || opd2_val)
                }
            },
            meerast::Expr::UopExpr { opd, uop } => match uop {
                meerast::Uop::Neg => {
                    let opd_val = match Worker::compute_val(opd, replica) {
                        message::Val::Int(i) => i,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Int(-opd_val)
                }
                meerast::Uop::Not => {
                    let opd_val = match Worker::compute_val(opd, replica) {
                        message::Val::Bool(b) => b,
                        _ => panic!("this indicates typechecking bugs"),
                    };
                    message::Val::Bool(!opd_val)
                }
            },
            meerast::Expr::IfExpr { cond, then, elze } => {
                let cond = match Worker::compute_val(cond, replica) {
                    message::Val::Bool(b) => b,
                    _ => panic!(),
                };
                if cond {
                    Worker::compute_val(then, replica)
                } else {
                    Worker::compute_val(elze, replica)
                }
            }
            meerast::Expr::Lambda { pars: _, body: _ } => message::Val::Lambda(expr.clone()),
        }
    }

    pub fn subst_pars_with_args(fun: &meerast::Expr, args: &Vec<meerast::Expr>) -> meerast::Expr {
        fn subst(expr: &mut meerast::Expr, ident_expr_map: &HashMap<String, &meerast::Expr>) {
            match expr {
                meerast::Expr::IdExpr { ident } => {
                    let ent = ident_expr_map.get(ident);
                    match ent {
                        Some(e) => *expr = e.deref().clone(),
                        None => {}
                    }
                }
                meerast::Expr::IntConst { val: _ } | meerast::Expr::BoolConst { val: _ } => {}
                meerast::Expr::Action { stmt } => {
                    let sgls = match stmt {
                        meerast::Stmt::Stmt { sgl_stmts } => sgl_stmts,
                    };
                    for sgl_stmt in sgls.iter_mut() {
                        match sgl_stmt {
                            meerast::SglStmt::Do { act } => {
                                subst(act, ident_expr_map);
                            }
                            meerast::SglStmt::Ass { dst: _, src } => {
                                subst(src, ident_expr_map);
                            }
                        }
                    }
                }
                meerast::Expr::Member {
                    srv_name: _,
                    member: _,
                } => panic!(),
                meerast::Expr::Apply { fun, args } => {
                    subst(fun, ident_expr_map);
                    for apply_arg in args.iter_mut() {
                        subst(apply_arg, ident_expr_map);
                    }
                }
                meerast::Expr::BopExpr { opd1, opd2, bop } => {
                    subst(opd1, ident_expr_map);
                    subst(opd2, ident_expr_map);
                }
                meerast::Expr::UopExpr { opd, uop } => {
                    subst(opd, ident_expr_map);
                }
                meerast::Expr::IfExpr { cond, then, elze } => {
                    subst(cond, ident_expr_map);
                    subst(then, ident_expr_map);
                    subst(elze, ident_expr_map);
                }
                meerast::Expr::Lambda { pars, body } => {
                    let mut par_names: HashSet<String> = HashSet::new();
                    for par in pars.iter() {
                        let name = match par {
                            meerast::Expr::IdExpr { ident } => ident.clone(),
                            _ => panic!(),
                        };
                        par_names.insert(name);
                    }
                    let mut body_map: HashMap<String, &meerast::Expr> = HashMap::new();
                    for (ident, arg_expr) in ident_expr_map.iter() {
                        if !par_names.contains(ident) {
                            body_map.insert(ident.clone(), arg_expr.deref());
                        }
                    }
                    subst(body, &body_map);
                }
            }
        }

        let pars = match fun {
            meerast::Expr::Lambda { pars: ps, body: _ } => ps,
            _ => panic!(),
        };
        let body = match fun {
            meerast::Expr::Lambda { pars: _, body: bd } => bd.deref(),
            _ => panic!(),
        };
        let mut par_arg_map: HashMap<String, &meerast::Expr> = HashMap::new();
        for (par, arg) in std::iter::zip(pars.iter(), args.iter()) {
            let par_ident = match par {
                meerast::Expr::IdExpr { ident } => ident.clone(),
                _ => panic!(),
            };
            par_arg_map.insert(par_ident, arg);
        }
        let mut substed_expr: Expr = body.clone();
        subst(&mut substed_expr, &par_arg_map);
        substed_expr
    }
}