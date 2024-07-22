pub mod backend;
pub mod frontend;

use backend::defworker_proc;
use backend::srvmanager_proc;
use frontend::meerast;
use frontend::parse;
use frontend::typecheck;
use std::{env, fs};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    /* let args: Vec<String> = env::args().collect();
    let file_name = &args[1];
    let src_prog = fs::read_to_string(file_name).expect("Unable to read file");
    let ast = parse::ProgramParser::new()
        .parse(src_prog.as_str())
        .expect("Parsing fail");
    let srvs = match ast {
        meerast::Program::Prog { services } => services,
    }; */
    let (tx, rx) = mpsc::channel(100);
    let _ = tokio::spawn(srvmanager_proc::manager_proc(tx));
    let _ = tokio::spawn(defworker_proc::defworker_proc(rx)).await;
}
