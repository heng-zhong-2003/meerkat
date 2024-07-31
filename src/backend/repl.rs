use std::collections::HashMap;
use std::f32::consts::PI;

use inline_colorization::*;
use tokio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

use crate::backend::{
    dependency,
    message::{Message, Val},
    srvmanager_proc::ServiceManager,
};
use crate::frontend::{
    meerast::{ReplInput, SglStmt},
    parse, typecheck,
};

pub async fn repl() {
    let mut srv_manager = ServiceManager::new();
    let repl_parser = parse::ReplInputParser::new();
    loop {
        let mut stdout = tokio::io::stdout();
        let stdin = tokio::io::stdin();
        /* display current environment */
        let mut curr_val_env: HashMap<String, Val> = HashMap::new();
        for (name, _) in srv_manager.typenv.iter() {
            let val_of_name = ServiceManager::retrieve_val(
                &srv_manager.worker_inboxes,
                &mut srv_manager.receiver_from_workers,
                name,
            )
            .await
            .expect("");
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
            crate::frontend::meerast::ReplInput::Do(sgl_stmt) => {
                todo!()
            }
            crate::frontend::meerast::ReplInput::Decl(decl) => {
                todo!()
            }
            crate::frontend::meerast::ReplInput::Update(_) => panic!(),
            crate::frontend::meerast::ReplInput::Open(_) => panic!(),
            crate::frontend::meerast::ReplInput::Close => panic!(),
            crate::frontend::meerast::ReplInput::Exit => std::process::exit(0),
        }
    }
}
