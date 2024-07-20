pub mod frontend;

use frontend::meerast;
use frontend::parse;
use frontend::typecheck;
use std::{env, fs};

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_name = &args[1];
    let src_prog = fs::read_to_string(file_name).expect("Unable to read file");
    let ast = parse::ProgramParser::new()
        .parse(src_prog.as_str())
        .expect("Parsing fail");
    // println!("{:?}", ast);
    let srvs = match ast {
        meerast::Program::Prog { services } => services,
    };
    typecheck::check_prog_test(&srvs[0]);
}
