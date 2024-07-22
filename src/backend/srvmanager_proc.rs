use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use crate::frontend::{meerast, typecheck};
use tokio::sync::mpsc;

pub async fn manager_proc(send_channel: mpsc::Sender<i32>) {
    let sigma_m: HashMap<String, typecheck::Type> = HashMap::new();
    let sigma_v: HashMap<String, typecheck::Type> = HashMap::new();
    let pub_access: HashMap<String, bool> = HashMap::new();
    let var_or_def: HashMap<String, VarOrDef> = HashMap::new();
}

pub enum VarOrDef {
    Var,
    Def,
}

pub enum DeclMsg {
    Import,
    Def {
        name: String,
        def_expr: meerast::Expr,
    },
    Var {
        name: String,
        init_state: meerast::Expr,
    },
}

pub fn interpret_decl(
    imported_services: &mut HashSet<String>,
    decl: &meerast::Decl,
) -> Result<DeclMsg, Box<dyn Error>> {
    match decl {
        meerast::Decl::Import { srv_name } => {
            imported_services.insert(srv_name.clone());
            Ok(DeclMsg::Import)
        }
        meerast::Decl::VarDecl { name, val } => {
            todo!()
        }
        meerast::Decl::DefDecl { name, val, is_pub } => todo!(),
    }
}
