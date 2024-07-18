trait AstNode {}

#[derive(Debug, Clone)]
pub enum ReplInput {
    Service(Service),
    Do(SglStmt),
    Decl(Decl),
    Open(String),
    Close,
    Exit,
}

#[derive(Debug, Clone)]
pub enum Program {
    Prog { services: Vec<Service> },
}

#[derive(Debug, Clone)]
pub enum Service {
    Svc { name: String, decls: Vec<Decl> },
}

#[derive(Debug, Clone)]
pub enum Decl {
    Import {
        svc_name: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Stmt { sgl_stmts: Vec<SglStmt> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SglStmt {
    Do { act: Expr },
    Ass { dst: Expr, src: Expr },
}

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
        svc_name: String,
        mbr_name: String,
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
        pars: Vec<String>,
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
