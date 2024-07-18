use std::{collections::HashMap, iter, ops::Deref, result};

use crate::frontend::meerast as ast;
use inline_colorization::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    Fun {
        par_types: Vec<Type>,
        ret_type: Box<Type>,
    },
    Action,
    Tyvar {
        name: String,
    },
    Meta {
        name: String,
    },
    Poly {
        tyvars: Vec<Type>,
        poly_type: Box<Type>,
    },
}

/* pub struct TypecheckEnv {
    pub sigma_ms: HashMap<String, HashMap<String, Type>>,
    pub sigma_vs: HashMap<String, HashMap<String, Type>>,
    pub pub_access: HashMap<String, HashMap<String, bool>>,
}

impl TypecheckEnv {
    pub fn new() -> TypecheckEnv {
        TypecheckEnv {
            sigma_ms: HashMap::new(),
            sigma_vs: HashMap::new(),
            pub_access: HashMap::new(),
        }
    }
} */

pub struct FreshTyvarGenerator {
    count: i32,
}

impl FreshTyvarGenerator {
    pub fn new(start: i32) -> FreshTyvarGenerator {
        FreshTyvarGenerator { count: start }
    }
    pub fn fresh(&mut self) -> Type {
        let ret = Type::Tyvar {
            name: format!("tyvar#{}", self.count),
        };
        self.count = self.count + 1;
        ret
    }
}

pub struct FreshMetaGenerator {
    count: i32,
}

impl FreshMetaGenerator {
    pub fn new(start: i32) -> FreshMetaGenerator {
        FreshMetaGenerator { count: start }
    }
    pub fn fresh(&mut self) -> Type {
        let ret = Type::Meta {
            name: format!("meta#{}", self.count),
        };
        self.count = self.count + 1;
        ret
    }
}

pub fn subst(
    gen_fresh_tyvar: &mut FreshTyvarGenerator,
    ty: &Type,
    sigma: &HashMap<String, Type>,
) -> Type {
    todo!()
}

pub fn unify(
    sigma_m: &mut HashMap<String, Type>,
    gen_fresh_meta: &mut FreshMetaGenerator,
    gen_fresh_tyvar: &mut FreshTyvarGenerator,
    ty1: &Type,
    ty2: &Type,
) -> Result<(), String> {
    match (ty1, ty2) {
        (Type::Int, Type::Int) | (Type::Bool, Type::Bool) | (Type::Action, Type::Action) => Ok(()),
        (
            Type::Fun {
                par_types: par_tys1,
                ret_type: ret_ty1,
            },
            Type::Fun {
                par_types: par_tys2,
                ret_type: ret_ty2,
            },
        ) => {
            for (i, j) in iter::zip(par_tys1.iter(), par_tys2.iter()) {
                let rslt = unify(sigma_m, gen_fresh_meta, gen_fresh_tyvar, i, j);
                match rslt {
                    Ok(_) => {}
                    Err(_) => {
                        return rslt;
                    }
                }
            }
            unify(sigma_m, gen_fresh_meta, gen_fresh_tyvar, ret_ty1, ret_ty2)
        }
        (
            Type::Poly {
                tyvars: alphas1,
                poly_type: u1,
            },
            Type::Poly {
                tyvars: alphas2,
                poly_type: u2,
            },
        ) => {
            if alphas1.len() != alphas2.len() {
                return Err(format!(
                    "{color_red}unification error, poly tyvars lengths differ"
                ));
            }
            let mut sigma: HashMap<String, Type> = HashMap::new();
            for (i, j) in iter::zip(alphas1.iter(), alphas2.iter()).into_iter() {
                let name = match i {
                    Type::Tyvar { name: s } => s.clone(),
                    _ => return Err(format!("non tyvar within polytype args")),
                };
                sigma.insert(name, j.clone());
            }
            let substed_u2 = subst(gen_fresh_tyvar, u2.deref(), &sigma);
            unify(sigma_m, gen_fresh_meta, gen_fresh_tyvar, u1, &substed_u2)
        }
        (Type::Tyvar { name: name1 }, Type::Tyvar { name: name2 }) => {
            todo!()
        }
        _ => Err(format!(
            "{color_red}unification error, {:?} and {:?} incompatible{color_reset}",
            ty1, ty2
        )),
    }
}
