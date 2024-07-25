pub mod backend;
pub mod frontend;

use backend::srvmanager_proc::ServiceManager;
use backend::srvmanager_proc::VarOrDef;
use backend::worker::Worker;
use frontend::meerast;
use frontend::parse;
use frontend::typecheck;
use std::collections::HashMap;
use std::{env, fs};

// #[tokio::main]
fn main() {
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
    let mut svc_manager = ServiceManage::new();
    ServiceManager::create_worker(
        "",
        VarOrDef::Def,
        svc_manager.sender_to_manager,
        subscribers,
        worker_inboxes,
        svc_manager.locks,
        svc_manager.typenv,
        svc_manager.var_or_def_env,
        svc_manager.dependgraph,
    )
}
