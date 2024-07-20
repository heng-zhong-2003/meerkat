use lalrpop_util::lalrpop_mod;

pub mod meerast;
lalrpop_mod!(pub parse, "/frontend/parse.rs");
pub mod typecheck;
