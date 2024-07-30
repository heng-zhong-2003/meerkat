use std::fmt::Display;

trait AstNode {}

impl AstNode for ReplInput {}
#[derive(Debug, Clone)]
pub enum ReplInput {
    Service(Service),
    Do(SglStmt),
    Decl(Decl),
    Update(Decl),
    Open(String),
    Close,
    Exit,
}

impl AstNode for Program {}
#[derive(Debug, Clone)]
pub enum Program {
    Prog { services: Vec<Service> },
}

impl AstNode for Service {}
#[derive(Debug, Clone)]
pub enum Service {
    Srv { name: String, decls: Vec<Decl> },
}

impl AstNode for Decl {}
#[derive(Debug, Clone)]
pub enum Decl {
    Import {
        srv_name: String,
    },
    VarDecl {
        name: String,
        val: Expr,
    },
    DefDecl {
        name: String,
        val: Expr,
        is_pub: bool,
    },
}

impl AstNode for Stmt {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Stmt { sgl_stmts: Vec<SglStmt> },
}

impl AstNode for SglStmt {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SglStmt {
    Do { act: Expr },
    Ass { dst: Expr, src: Expr },
}

impl AstNode for Expr {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    IdExpr {
        ident: String,
    },
    IntConst {
        val: i32,
    },
    BoolConst {
        val: bool,
    },
    Action {
        stmt: Stmt,
    },
    Member {
        srv_name: String,
        member: Box<Expr>,
    },
    Apply {
        fun: Box<Expr>,
        args: Vec<Expr>,
    },
    BopExpr {
        opd1: Box<Expr>,
        opd2: Box<Expr>,
        bop: Binop,
    },
    UopExpr {
        opd: Box<Expr>,
        uop: Uop,
    },
    IfExpr {
        cond: Box<Expr>,
        then: Box<Expr>,
        elze: Box<Expr>,
    },
    Lambda {
        pars: Vec<Expr>,
        body: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Uop {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binop {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Gt,
    And,
    Or,
}
