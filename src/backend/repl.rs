use std::collections::HashMap;
use std::f32::consts::PI;

use inline_colorization::*;
use tokio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

use crate::backend::{
    dependency,
    message::{Message, Val},
    srvmanager_proc::{ServiceManager, VarOrDef},
};
use crate::frontend::meerast;
use crate::frontend::{
    meerast::{Decl, ReplInput, SglStmt},
    parse,
    typecheck::{self, FreshMetaGenerator, FreshTyvarGenerator, Type},
};

pub async fn repl() {
    let mut srv_manager = ServiceManager::new();
    let repl_parser = parse::ReplInputParser::new();
    let mut sigma_m: HashMap<String, Type> = HashMap::new();
    let mut sigma_v: HashMap<String, Type> = HashMap::new();
    let mut pub_access: HashMap<String, bool> = HashMap::new();
    let mut gen_fresh_meta = FreshMetaGenerator::new("default", 0);
    let mut gen_fresh_tyvar = FreshTyvarGenerator::new("default", 0);
    loop {
        let mut stdout = tokio::io::stdout();
        let stdin = tokio::io::stdin();
        /* display current environment */
        let mut curr_val_env: HashMap<String, _> = HashMap::new();
        // tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        for (name, _) in srv_manager.typenv.iter() {
            let val_of_name = ServiceManager::retrieve_val(
                &srv_manager.worker_inboxes,
                &mut srv_manager.receiver_from_workers,
                name,
            )
            .await;
            curr_val_env.insert(name.clone(), val_of_name);
        }
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
        println!("dependency: {:?}", srv_manager.dependgraph);

        /* get and process user input */
        let _ = stdout
            .write_all(
                &format!("{color_green}{style_bold}λ> {style_reset}{color_reset}").as_bytes(),
            )
            .await
            .expect("tokio output error");
        let _ = stdout.flush().await.unwrap();

        let reader = tokio::io::BufReader::new(stdin);
        let mut lines = reader.lines();
        let command_string = lines.next_line().await.expect("").expect("");

        /* let _ = stdout
        .write_all(&format!("prev input: {}\n", command_string).as_bytes())
        .await
        .expect("tokio output error"); */

        let command_ast = match repl_parser.parse(&command_string) {
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
            crate::frontend::meerast::ReplInput::Service(_) => panic!(),
            crate::frontend::meerast::ReplInput::Do(sgl_stmt) => match sgl_stmt {
                SglStmt::Do { act } => todo!(),
                SglStmt::Ass { dst, src } => {
                    let dst_name = match dst {
                        meerast::Expr::IdExpr { ident } => ident,
                        _ => panic!(),
                    };
                    let msg = Message::AssignVar {
                        var_name: dst_name.clone(),
                        new_val_expr: src,
                    };
                    let worker_addr = srv_manager.worker_inboxes.get(&dst_name).unwrap();
                    let _ = worker_addr.send(msg).await.expect("");
                }
            },
            crate::frontend::meerast::ReplInput::Decl(decl) => {
                /* type check decl */
                match typecheck::check_decl(
                    &mut sigma_v,
                    &mut sigma_m,
                    &mut pub_access,
                    &mut gen_fresh_meta,
                    &mut gen_fresh_tyvar,
                    &decl,
                ) {
                    Ok(_) => {}
                    Err(_) => {
                        let _ = stdout
                            .write_all(&format!("{color_red}type error{color_reset}\n").as_bytes())
                            .await
                            .expect("tokio output error");
                        continue;
                    }
                }

                match decl {
                    Decl::Import { srv_name: _ } => panic!(),
                    Decl::VarDecl { name, val } => {
                        ServiceManager::create_worker(
                            &name,
                            VarOrDef::Var,
                            &vec![],
                            None,
                            srv_manager.sender_to_manager.clone(),
                            &mut srv_manager.worker_inboxes,
                            &mut srv_manager.locks,
                            &mut srv_manager.typenv,
                            &mut srv_manager.var_or_def_env,
                            &mut srv_manager.dependgraph,
                        )
                        .await;
                        ServiceManager::init_var_worker(
                            &mut srv_manager.worker_inboxes,
                            &name,
                            val,
                        )
                        .await;
                    }
                    Decl::DefDecl {
                        ref name,
                        ref val,
                        is_pub,
                    } => {
                        let mut temp_dependgraph = srv_manager.dependgraph.clone();
                        dependency::decl_dependency(&mut temp_dependgraph, &decl);
                        match dependency::check_cyclic(&temp_dependgraph) {
                            Ok(_) => {
                                srv_manager.dependgraph = temp_dependgraph;
                            }
                            Err(_) => {
                                let _ = stdout
                                    .write_all(
                                        &format!(
                                            "{color_red}cyclic dependency error{color_reset}\n"
                                        )
                                        .as_bytes(),
                                    )
                                    .await
                                    .expect("tokio output error");
                                continue;
                            }
                        }
                        ServiceManager::create_worker(
                            name,
                            VarOrDef::Def,
                            &srv_manager
                                .dependgraph
                                .get(name)
                                .expect("")
                                .into_iter()
                                .map(|x| x.clone())
                                .collect(),
                            Some(val.clone()),
                            srv_manager.sender_to_manager.clone(),
                            &mut srv_manager.worker_inboxes,
                            &mut srv_manager.locks,
                            &mut srv_manager.typenv,
                            &mut srv_manager.var_or_def_env,
                            &mut srv_manager.dependgraph,
                        )
                        .await;
                    }
                }
            }
            crate::frontend::meerast::ReplInput::Update(_) => panic!(),
            crate::frontend::meerast::ReplInput::Open(_) => panic!(),
            crate::frontend::meerast::ReplInput::Close => panic!(),
            crate::frontend::meerast::ReplInput::Exit => std::process::exit(0),
        }
    }
}
