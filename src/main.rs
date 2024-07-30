pub mod backend;
pub mod frontend;

use backend::srvmanager_proc::ServiceManager;
use backend::srvmanager_proc::VarOrDef;
use backend::worker::Worker;
use frontend::meerast;
use frontend::parse;
use frontend::typecheck;
use inline_colorization::*;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Arc;
use std::{env, fs};
use tokio::io;
use tracing_subscriber::fmt::MakeWriter;

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
    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        tracing_appender::rolling::Rotation::NEVER,
        "./",
        "log.txt",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .with_writer(non_blocking)
        .event_format(
            tracing_subscriber::fmt::format()
                .compact()
                .with_level(true)
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false),
        )
        .pretty()
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Unable to set global subscriber");

    let mut svc_manager = ServiceManager::new();

    ServiceManager::create_worker(
        "c",
        VarOrDef::Def,
        svc_manager.sender_to_manager.clone(),
        &HashSet::new(),
        &mut svc_manager.worker_inboxes,
        &mut svc_manager.locks,
        &mut svc_manager.typenv,
        &mut svc_manager.var_or_def_env,
        &mut svc_manager.dependgraph,
    );

    ServiceManager::create_worker(
        "a",
        VarOrDef::Def,
        svc_manager.sender_to_manager.clone(),
        &HashSet::from_iter(vec!["c".to_string()].into_iter()),
        &mut svc_manager.worker_inboxes,
        &mut svc_manager.locks,
        &mut svc_manager.typenv,
        &mut svc_manager.var_or_def_env,
        &mut svc_manager.dependgraph,
    );

    ServiceManager::create_worker(
        "b",
        VarOrDef::Def,
        svc_manager.sender_to_manager.clone(),
        &HashSet::from_iter(vec!["c".to_string()].into_iter()),
        &mut svc_manager.worker_inboxes,
        &mut svc_manager.locks,
        &mut svc_manager.typenv,
        &mut svc_manager.var_or_def_env,
        &mut svc_manager.dependgraph,
    );

    ServiceManager::create_worker(
        "x",
        VarOrDef::Var,
        svc_manager.sender_to_manager.clone(),
        &HashSet::from_iter(vec!["a".to_string(), "b".to_string()].into_iter()),
        // &HashSet::new(),
        &mut svc_manager.worker_inboxes,
        &mut svc_manager.locks,
        &mut svc_manager.typenv,
        &mut svc_manager.var_or_def_env,
        &mut svc_manager.dependgraph,
    );

    let _ = ServiceManager::init_def_worker(
        &mut svc_manager.worker_inboxes,
        "c",
        meerast::Expr::BopExpr {
            opd1: Box::new(meerast::Expr::IdExpr {
                ident: String::from("a"),
            }),
            opd2: Box::new(meerast::Expr::IdExpr {
                ident: String::from("b"),
            }),
            bop: meerast::Binop::Add,
        },
    )
    .await;

    let _ = ServiceManager::init_def_worker(
        &mut svc_manager.worker_inboxes,
        "a",
        meerast::Expr::BopExpr {
            opd1: Box::new(meerast::Expr::IdExpr {
                ident: String::from("x"),
            }),
            opd2: Box::new(meerast::Expr::IntConst { val: 1 }),
            bop: meerast::Binop::Add,
        },
    )
    .await;

    let _ = ServiceManager::init_def_worker(
        &mut svc_manager.worker_inboxes,
        "b",
        meerast::Expr::BopExpr {
            opd1: Box::new(meerast::Expr::IdExpr {
                ident: String::from("x"),
            }),
            opd2: Box::new(meerast::Expr::IntConst { val: 2 }),
            bop: meerast::Binop::Mul,
        },
    )
    .await;

    let _ = ServiceManager::init_var_worker(
        &mut svc_manager.worker_inboxes,
        "x",
        meerast::Expr::IntConst { val: 1 },
    )
    .await;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let xval = ServiceManager::retrieve_val(
        &svc_manager.worker_inboxes,
        &mut svc_manager.receiver_from_workers,
        "x",
    )
    .await;
    let aval = ServiceManager::retrieve_val(
        &svc_manager.worker_inboxes,
        &mut svc_manager.receiver_from_workers,
        "a",
    )
    .await;
    let bval = ServiceManager::retrieve_val(
        &svc_manager.worker_inboxes,
        &mut svc_manager.receiver_from_workers,
        "b",
    )
    .await;
    let cval = ServiceManager::retrieve_val(
        &svc_manager.worker_inboxes,
        &mut svc_manager.receiver_from_workers,
        "c",
    )
    .await;

    println!("x: {:?}, a: {:?}, b: {:?}, c: {:?}", xval, aval, bval, cval);
}
