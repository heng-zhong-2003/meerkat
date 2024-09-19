use crate::{
    backend::{
        dependency,
        manager::Manager,
        message::Val,
        transaction::{Txn, TxnId, WriteToName},
    },
    frontend::{
        meerast::{Decl, Expr, ReplInput, SglStmt},
        parse::ReplInputParser,
        typecheck::{self, FreshMetaGenerator, FreshTyvarGenerator, Type},
    },
};

use inline_colorization::*;

use std::collections::HashMap;

use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt},
};

use super::message::Message;

pub async fn repl() {
    let mut manager = Manager::new();
    let parser = ReplInputParser::new();
    let mut sigma_m: HashMap<String, Type> = HashMap::new();
    let mut sigma_v: HashMap<String, Type> = HashMap::new();
    let mut pub_access: HashMap<String, bool> = HashMap::new();
    let mut gen_fresh_meta = FreshMetaGenerator::new("default", 0);
    let mut gen_fresh_tyvar = FreshTyvarGenerator::new("default", 0);
    loop {
        let mut stdout = tokio::io::stdout();
        let stdin = tokio::io::stdin();

        // output current context and prompt
        let mut curr_val_env: HashMap<String, Option<Val>> = HashMap::new();
        for (name, _) in manager.type_env.iter() {
            let val_of_name = Manager::retrieve_val_of(
                name,
                &manager.workers_inboxes_senders,
                &mut manager.receiver_from_workers,
            )
            .await;
            println!("insert {:?}, {:?}", name, val_of_name);
            curr_val_env.insert(name.clone(), val_of_name);
        }
        println!("curr_val_env: {:?}", curr_val_env);
        let _ = stdout
            .write_all(&format!("{color_green}current environment{color_reset}\n").as_bytes())
            .await
            .expect("tokio output error");
        for (name, val) in curr_val_env.iter() {
            let _ = stdout
                .write_all(&format!("{color_green}{}: {:?}{color_reset}\n", name, val).as_bytes())
                .await
                .expect("tokio output error");
        }

        let _ = stdout
            .write_all(
                &format!("{color_green}{style_bold}λ> {style_reset}{color_reset}").as_bytes(),
            )
            .await
            .expect("tokio output error");
        let _ = stdout.flush().await.unwrap();

        // user input
        let reader = tokio::io::BufReader::new(stdin);
        let mut lines = reader.lines();
        let command_string = lines.next_line().await.expect("").expect("");

        let command_ast = match parser.parse(&command_string) {
            Ok(ast) => ast,
            Err(_) => {
                let _ = stdout
                    .write_all(&format!("{color_red}syntax error{color_reset}\n").as_bytes())
                    .await
                    .expect("tokio output error");
                continue;
            }
        };

        match command_ast {
            ReplInput::Service(_) => panic!(),
            ReplInput::Do(sgl_stmt) => match sgl_stmt {
                SglStmt::Do { act } => todo!(),
                SglStmt::Ass { dst, src } => {
                    let dst_name = match dst {
                        Expr::IdExpr { ident } => ident,
                        _ => panic!(),
                    };
                    /* let names_in_src = Manager::get_names_in_expr(&src);
                    let mut names_to_values: HashMap<String, Val> = HashMap::new();
                    for nm in names_in_src.into_iter() {
                        let val_of_nm = Manager::retrieve_val_of(
                            &nm,
                            &manager.workers_inboxes_senders,
                            &mut manager.receiver_from_workers,
                        )
                        .await
                        .unwrap();
                        names_to_values.insert(nm, val_of_nm);
                    }
                    let substed_src =
                        Manager::subst_idents_in_expr_for_vals(&src, &names_to_values); */
                    let assign_txn = Txn {
                        id: TxnId::new(),
                        writes: vec![WriteToName {
                            name: dst_name,
                            expr: src,
                        }],
                    };
                    Manager::handle_transaction(
                        &assign_txn,
                        &manager.var_or_def_env,
                        &mut manager.workers_inboxes_senders,
                        &mut manager.receiver_from_workers,
                    )
                    .await;
                }
            },
            ReplInput::Decl(decl) => match decl {
                Decl::Import { srv_name: _ } => panic!(),
                Decl::VarDecl { name, val } => {
                    Manager::create_var_worker(
                        &name,
                        manager.sender_to_manager.clone(),
                        &mut manager.workers_inboxes_senders,
                        &mut manager.type_env,
                        &mut manager.var_or_def_env,
                    )
                    .await;
                    let assign_txn = Txn {
                        id: TxnId::new(),
                        writes: vec![WriteToName {
                            name: name,
                            expr: val,
                        }],
                    };
                    Manager::handle_transaction(
                        &assign_txn,
                        &manager.var_or_def_env,
                        &mut manager.workers_inboxes_senders,
                        &mut manager.receiver_from_workers,
                    )
                    .await;
                }
                Decl::DefDecl {
                    ref name,
                    ref val,
                    is_pub: _,
                } => {
                    let mut temp_dep_graph = manager.dependency_graph.clone();
                    dependency::decl_dependency(&mut temp_dep_graph, &decl);
                    match dependency::check_cyclic(&temp_dep_graph) {
                        Ok(_) => manager.dependency_graph = temp_dep_graph,
                        Err(_) => {
                            let _ = stdout
                                .write_all(
                                    &format!("{color_red}cyclic dependency error{color_reset}\n")
                                        .as_bytes(),
                                )
                                .await
                                .expect("tokio output error");
                            continue;
                        }
                    }
                    let mut replica: HashMap<String, Option<Val>> = HashMap::new();
                    for dep in manager.dependency_graph.get(name).unwrap().iter() {
                        replica.insert(dep.clone(), None);
                    }
                    Manager::create_def_worker(
                        name,
                        manager.sender_to_manager.clone(),
                        val.clone(),
                        replica,
                        todo!(),
                        &mut manager.workers_inboxes_senders,
                        &mut manager.type_env,
                        &mut manager.var_or_def_env,
                    )
                    .await;
                }
            },
            ReplInput::Update(_) => panic!(),
            ReplInput::Open(_) => panic!(),
            ReplInput::Close => panic!(),
            ReplInput::Exit => {
                std::process::exit(0);
            }
        }
    }
}
