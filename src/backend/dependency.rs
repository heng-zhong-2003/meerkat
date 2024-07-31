use std::collections::{HashMap, HashSet};

use crate::frontend::meerast;

pub fn decl_dependency(
    dependency_graph: &mut HashMap<String, HashSet<String>>,
    decl: &meerast::Decl,
) {
    match decl {
        meerast::Decl::Import { srv_name: _ } => panic!("not yet support multi service"),
        meerast::Decl::VarDecl { name: _, val: _ } => {}
        meerast::Decl::DefDecl {
            name,
            val,
            is_pub: _,
        } => {
            let mut dependency_set: HashSet<String> = HashSet::new();
            expr_dependency(&mut dependency_set, val);
            dependency_graph.insert(name.clone(), dependency_set);
        }
    }
}

pub fn expr_dependency(dependency_set: &mut HashSet<String>, expr: &meerast::Expr) {
    match expr {
        meerast::Expr::IdExpr { ident } => {
            dependency_set.insert(ident.clone());
        }
        meerast::Expr::IntConst { val: _ } | meerast::Expr::BoolConst { val: _ } => {}
        meerast::Expr::Action { stmt } => {
            let sgls = match stmt {
                meerast::Stmt::Stmt { sgl_stmts } => sgl_stmts,
            };
            for sgl in sgls.iter() {
                match sgl {
                    meerast::SglStmt::Do { act } => {
                        expr_dependency(dependency_set, act);
                    }
                    meerast::SglStmt::Ass { dst: _, src } => {
                        expr_dependency(dependency_set, src);
                    }
                }
            }
        }
        meerast::Expr::Member {
            srv_name: _,
            member: _,
        } => panic!("not yet support multi service"),
        meerast::Expr::Apply { fun, args } => {
            expr_dependency(dependency_set, fun);
            for arg_expr in args.iter() {
                expr_dependency(dependency_set, arg_expr);
            }
        }
        meerast::Expr::BopExpr { opd1, opd2, bop: _ } => {
            expr_dependency(dependency_set, opd1);
            expr_dependency(dependency_set, opd2);
        }
        meerast::Expr::UopExpr { opd, uop: _ } => {
            expr_dependency(dependency_set, opd);
        }
        meerast::Expr::IfExpr { cond, then, elze } => {
            expr_dependency(dependency_set, cond);
            expr_dependency(dependency_set, then);
            expr_dependency(dependency_set, elze);
        }
        meerast::Expr::Lambda { pars, body } => {
            let mut par_names: HashSet<String> = HashSet::new();
            for par in pars.iter() {
                let par_name = match par {
                    meerast::Expr::IdExpr { ident } => ident.clone(),
                    _ => panic!(),
                };
                par_names.insert(par_name);
            }
            expr_dependency(dependency_set, body);
            *dependency_set = dependency_set.difference(&par_names).cloned().collect();
        }
    }
}

pub fn check_cyclic(dependency_graph: &HashMap<String, HashSet<String>>) -> Result<(), String> {
    fn rec_check_name(
        curr_name: &str,
        dependency_graph: &HashMap<String, HashSet<String>>,
        encountered: &mut HashSet<String>,
    ) -> Result<(), String> {
        if encountered.contains(curr_name) {
            Err(String::from("cyclic dependency"))
        } else {
            encountered.insert(curr_name.to_string());
            let succs = dependency_graph.get(curr_name);
            if let Some(succs) = succs {
                for succ in succs.iter() {
                    let _ = rec_check_name(succ, dependency_graph, encountered)?;
                }
                Ok(())
            } else {
                Ok(())
            }
        }
    }
    for (ident, _) in dependency_graph.iter() {
        let mut encountered: HashSet<String> = HashSet::new();
        let _ = rec_check_name(ident, dependency_graph, &mut encountered);
    }
    Ok(())
}
