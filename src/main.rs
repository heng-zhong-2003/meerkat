pub mod backend;
pub mod frontend;

use backend::srvmanager_proc::ServiceManager;
use backend::srvmanager_proc::VarOrDef;
use backend::worker::Worker;
use frontend::meerast;
use frontend::parse;
use frontend::typecheck;
use std::collections::{HashMap, HashSet};
use std::{env, fs};

#[tokio::main]
async fn main() {
    // let args: Vec<String> = env::args().collect();
    // let file_name = &args[1];
    // let src_prog = fs::read_to_string(file_name).expect("Unable to read file");
    // // let ast = parse::ProgramParser::new()
    // //     .parse(src_prog.as_str())
    // //     .expect("Parsing fail");
    // // let srvs = match ast {
    // //     meerast::Program::Prog { services } => services,
    // // };
    // let ast = parse::ExprParser::new().parse(src_prog.as_str()).expect("");
    // let val = Worker::compute_val(&ast, &HashMap::new());
    // println!("{:?}", val);
    // /* let (tx, rx) = mpsc::channel(100);
    // let _ = tokio::spawn(srvmanager_proc::manager_proc(tx));
    // let _ = tokio::spawn(defworker_proc::defworker_proc(rx)).await; */


    // example 1
    // var       x
    //         /   \
    //        /     \
    // def  a = x+1  b = x*2 
    //        \     /
    //         \   /
    // def       c = a+b
    let mut svc_manager = ServiceManager::new();

    ServiceManager::create_worker(
        "x",
        VarOrDef::Var,
        &Vec::new(),
        None,
        svc_manager.sender_to_manager.clone(),
        &mut svc_manager.worker_inboxes,
        &mut svc_manager.locks,
        &mut svc_manager.typenv,
        &mut svc_manager.var_or_def_env,
        &mut svc_manager.dependgraph,
    ).await;

    ServiceManager::create_worker(
        "a",
        VarOrDef::Def,
        &vec!["x".to_string()],
        Some(
            meerast::Expr::BopExpr {
                opd1: Box::new(meerast::Expr::IdExpr { ident: String::from("x") }),
                opd2: Box::new(meerast::Expr::IntConst { val: 1 }),
                bop: meerast::Binop::Add,
            }
        ),
        svc_manager.sender_to_manager.clone(),
        &mut svc_manager.worker_inboxes,
        &mut svc_manager.locks,
        &mut svc_manager.typenv,
        &mut svc_manager.var_or_def_env,
        &mut svc_manager.dependgraph,
    ).await;

    ServiceManager::create_worker(
        "b",
        VarOrDef::Def, 
        &vec!["x".to_string()],
        Some(
            meerast::Expr::BopExpr {
                opd1: Box::new(meerast::Expr::IdExpr { ident: String::from("x") }),
                opd2: Box::new(meerast::Expr::IntConst { val: 2 }),
                bop: meerast::Binop::Mul,
            }
        ),
        svc_manager.sender_to_manager.clone(),
        &mut svc_manager.worker_inboxes,
        &mut svc_manager.locks,
        &mut svc_manager.typenv,
        &mut svc_manager.var_or_def_env,
        &mut svc_manager.dependgraph,
    ).await;

    ServiceManager::create_worker(
        "c",
        VarOrDef::Def,
        &vec!["a".to_string(), "b".to_string()],
        Some( 
            meerast::Expr::BopExpr {
            opd1: Box::new(meerast::Expr::IdExpr { ident: String::from("a") }),
            opd2: Box::new(meerast::Expr::IdExpr { ident: String::from("b") }),
            bop: meerast::Binop::Add,
            }
        ),
        svc_manager.sender_to_manager.clone(),
        &mut svc_manager.worker_inboxes,
        &mut svc_manager.locks,
        &mut svc_manager.typenv,
        &mut svc_manager.var_or_def_env,
        &mut svc_manager.dependgraph,
    ).await;
    

    println!("start initialize worker x");
    let _ = ServiceManager::init_var_worker(
        &mut svc_manager.worker_inboxes, 
        "x", 
        meerast::Expr::IntConst { val: 1 }
    ).await;  

    // println!("start initialize worker a");
    // let _ = ServiceManager::init_def_worker(
    //     &mut svc_manager.worker_inboxes, 
    //     "a",
    //     meerast::Expr::BopExpr {
    //         opd1: Box::new(meerast::Expr::IdExpr { ident: String::from("x") }),
    //         opd2: Box::new(meerast::Expr::IntConst { val: 1 }),
    //         bop: meerast::Binop::Add,
    //     },
    // ).await;

    // println!("start initialize worker b");
    // let _ = ServiceManager::init_def_worker(
    //     &mut svc_manager.worker_inboxes, 
    //     "b",
    //     meerast::Expr::BopExpr {
    //         opd1: Box::new(meerast::Expr::IdExpr { ident: String::from("x") }),
    //         opd2: Box::new(meerast::Expr::IntConst { val: 2 }),
    //         bop: meerast::Binop::Mul,
    //     },
    // ).await;

    // println!("start initialize worker c");
    // let _ = ServiceManager::init_def_worker(
    //     &mut svc_manager.worker_inboxes, 
    //     "c",
    //     meerast::Expr::BopExpr {
    //         opd1: Box::new(meerast::Expr::IdExpr { ident: String::from("a") }),
    //         opd2: Box::new(meerast::Expr::IdExpr { ident: String::from("b") }),
    //         bop: meerast::Binop::Add,
    //     },
    // ).await;

    // let xval = ServiceManager::retrieve_val(
    //     &svc_manager.worker_inboxes, 
    //     &mut svc_manager.receiver_from_workers,
    //     "x",
    // ).await;
    
    // let aval = ServiceManager::retrieve_val(
    //     &svc_manager.worker_inboxes, 
    //     &mut svc_manager.receiver_from_workers,
    //     "a",
    // );
    // let bval = ServiceManager::retrieve_val(
    //     &svc_manager.worker_inboxes, 
    //     &mut svc_manager.receiver_from_workers,
    //     "b",
    // ).await;
    
    let cval = ServiceManager::retrieve_val(
        &svc_manager.worker_inboxes, 
        &mut svc_manager.receiver_from_workers,
        "c",
    ).await;
    // println!("x: {:?}, a: {:?}, b: {:?}, c: {:?}", xval, aval, bval, cval);
}
