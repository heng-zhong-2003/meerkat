use lalrpop_util::lalrpop_mod;

pub mod meerast;
lalrpop_mod!(pub parse);
pub mod typecheck;
