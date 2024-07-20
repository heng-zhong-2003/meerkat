use std::{
    collections::{HashMap, HashSet},
    iter,
    ops::Deref,
};

use crate::meerast;
use inline_colorization::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Bool,
    Unit,
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
    srv: String,
    count: i32,
}

impl FreshTyvarGenerator {
    pub fn new(srv: &str, start: i32) -> FreshTyvarGenerator {
        FreshTyvarGenerator {
            srv: srv.to_string(),
            count: start,
        }
    }
    pub fn fresh(&mut self) -> Type {
        let ret = Type::Tyvar {
            name: format!("{}#tyvar#{}", self.srv, self.count),
        };
        self.count = self.count + 1;
        ret
    }
}

pub struct FreshMetaGenerator {
    srv: String,
    count: i32,
}

impl FreshMetaGenerator {
    pub fn new(srv: &str, start: i32) -> FreshMetaGenerator {
        FreshMetaGenerator {
            srv: srv.to_string(),
            count: start,
        }
    }
    pub fn fresh(&mut self) -> Type {
        let ret = Type::Meta {
            name: format!("{}#meta#{}", self.srv, self.count),
        };
        self.count = self.count + 1;
        ret
    }
}

pub fn lookup_sigma_m_bottom(sigma_m: &HashMap<String, Type>, meta_name: &str) -> Option<Type> {
    let val_for_meta = sigma_m.get(meta_name);
    match val_for_meta {
        Some(val) => match val {
            Type::Meta { name: next_meta } => {
                if sigma_m.contains_key(next_meta) {
                    lookup_sigma_m_bottom(sigma_m, next_meta)
                } else {
                    Some(Type::Meta {
                        name: next_meta.clone(),
                    })
                }
            }
            _ => Some(val.clone()),
        },
        None => None,
    }
}

pub fn subst(
    gen_fresh_tyvar: &mut FreshTyvarGenerator,
    ty: &Type,
    tyvar_to_type: &HashMap<String, Type>,
    sigma_m: &mut HashMap<String, Type>,
) -> Type {
    match ty {
        Type::Int | Type::Bool | Type::Unit | Type::Action => ty.clone(),
        Type::Tyvar { name: alpha } => match tyvar_to_type.get(alpha) {
            Some(t) => t.clone(),
            None => ty.clone(),
        },
        Type::Fun {
            par_types: pars,
            ret_type: ret,
        } => {
            let mut rslt_par_types: Vec<Type> = vec![];
            for i in pars.iter() {
                rslt_par_types.push(subst(gen_fresh_tyvar, i, tyvar_to_type, sigma_m));
            }
            let rslt_ret_type = subst(gen_fresh_tyvar, ret, tyvar_to_type, sigma_m);
            Type::Fun {
                par_types: rslt_par_types,
                ret_type: Box::new(rslt_ret_type),
            }
        }
        Type::Poly {
            tyvars: poly_pars,
            poly_type: u,
        } => {
            let mut gammas: Vec<Type> = vec![];
            let mut tyvars_old_to_new: HashMap<String, Type> = HashMap::new();
            for i in poly_pars.iter() {
                let new_tyvar = gen_fresh_tyvar.fresh();
                gammas.push(new_tyvar.clone());
                tyvars_old_to_new.insert(
                    match i {
                        Type::Tyvar { name: s } => s.clone(),
                        _ => panic!(),
                    },
                    new_tyvar,
                );
            }
            let u1 = subst(gen_fresh_tyvar, u, &tyvars_old_to_new, sigma_m);
            Type::Poly {
                tyvars: gammas,
                poly_type: Box::new(subst(gen_fresh_tyvar, &u1, tyvar_to_type, sigma_m)),
            }
        }
        Type::Meta { name: alpha } => {
            let sigma_m_alpha = lookup_sigma_m_bottom(sigma_m, alpha);
            match sigma_m_alpha {
                Some(t) => subst(gen_fresh_tyvar, &t, tyvar_to_type, sigma_m),
                None => ty.clone(),
            }
        }
        _ => panic!(),
    }
}

pub fn all_metas_in_type(ty: &Type) -> HashSet<Type> {
    let mut rslt: HashSet<Type> = HashSet::new();
    match ty {
        Type::Int | Type::Bool | Type::Unit | Type::Action | Type::Tyvar { name: _ } => {}
        Type::Fun {
            par_types,
            ret_type,
        } => {
            for i in par_types.iter() {
                let par_metas = all_metas_in_type(i);
                rslt = rslt.union(&par_metas).cloned().collect();
            }
            let ret_metas = all_metas_in_type(ret_type);
            rslt = rslt.union(&ret_metas).cloned().collect();
        }
        Type::Meta { name } => {
            rslt.insert(Type::Meta { name: name.clone() });
        }
        Type::Poly {
            tyvars: _,
            poly_type,
        } => {
            let poly_body_metas = all_metas_in_type(poly_type);
            rslt = rslt.union(&poly_body_metas).cloned().collect();
        }
        _ => panic!(),
    }
    rslt
}

pub fn all_metas_in_type_bottom(sigma_m: &HashMap<String, Type>, ty: &Type) -> HashSet<Type> {
    fn get_bottom_of_meta(sigma_m: &HashMap<String, Type>, meta: &str) -> Option<Type> {
        let val_for_meta = sigma_m.get(meta);
        match val_for_meta {
            Some(val) => match val {
                Type::Meta { name } => get_bottom_of_meta(sigma_m, name),
                _ => None,
            },
            None => Some(Type::Meta {
                name: meta.to_string(),
            }),
        }
    }
    let mut rslt: HashSet<Type> = HashSet::new();
    match ty {
        Type::Int | Type::Bool | Type::Unit | Type::Action | Type::Tyvar { name: _ } => {}
        Type::Fun {
            par_types,
            ret_type,
        } => {
            for i in par_types.iter() {
                let bottom_metas_in_par = all_metas_in_type_bottom(sigma_m, i);
                rslt = rslt.union(&bottom_metas_in_par).cloned().collect();
            }
            let bottom_metas_in_ret = all_metas_in_type_bottom(sigma_m, &ret_type);
            rslt = rslt.union(&bottom_metas_in_ret).cloned().collect();
        }
        Type::Meta { name } => {
            let bottom_of_this_meta = get_bottom_of_meta(sigma_m, name);
            match bottom_of_this_meta {
                Some(m) => {
                    rslt.insert(m);
                }
                None => {}
            }
        }
        Type::Poly {
            tyvars: _,
            poly_type,
        } => {
            let bottom_metas_in_poly_body = all_metas_in_type_bottom(sigma_m, poly_type);
            rslt = rslt.union(&bottom_metas_in_poly_body).cloned().collect();
        }
        _ => panic!(),
    }
    rslt
}

pub fn meta_is_in_sigma_v(
    meta: &str,
    sigma_m: &HashMap<String, Type>,
    sigma_v: &HashMap<String, Type>,
) -> bool {
    fn meta_is_in_type_rec_sigma_m(sigma_m: &HashMap<String, Type>, meta: &str, ty: &Type) -> bool {
        let all_metas_in_ty = all_metas_in_type(ty);
        if all_metas_in_ty.contains(&Type::Meta {
            name: meta.to_string(),
        }) {
            return true;
        }
        for m in all_metas_in_ty.iter() {
            let m_name = match m {
                Type::Meta { name: n } => n,
                _ => panic!(),
            };
            let next_m_in_sigma_m = sigma_m.get(m_name).expect("");
            if meta_is_in_type_rec_sigma_m(sigma_m, meta, next_m_in_sigma_m) {
                return true;
            }
        }
        false
    }
    if sigma_v.contains_key(meta) {
        return true;
    }
    for (_, corresponding_ty_in_sigma_v) in sigma_v.iter() {
        if meta_is_in_type_rec_sigma_m(sigma_m, meta, corresponding_ty_in_sigma_v) {
            return true;
        }
    }
    false
}

pub fn generalize(
    sigma_m: &mut HashMap<String, Type>,
    sigma_v: &HashMap<String, Type>,
    ungeneralizeded_type: &Type,
    gen_fresh_tyvar: &mut FreshTyvarGenerator,
) -> Type {
    let bottom_metas_in_ungen = all_metas_in_type_bottom(sigma_m, ungeneralizeded_type);
    let mut metas_in_ungen_not_in_sigma_v: HashSet<Type> = HashSet::new();
    for meta in bottom_metas_in_ungen.iter() {
        let meta_name = match meta {
            Type::Meta { name } => name.clone(),
            _ => panic!(),
        };
        if !meta_is_in_sigma_v(&meta_name, sigma_m, sigma_v) {
            metas_in_ungen_not_in_sigma_v.insert(Type::Meta { name: meta_name });
        }
    }
    let mut new_tyvars_for_gen: Vec<Type> = vec![];
    for alpha_i in metas_in_ungen_not_in_sigma_v.iter() {
        let alpha_i_name = match alpha_i {
            Type::Meta { name } => name.clone(),
            _ => panic!(),
        };
        let new_tyvar = gen_fresh_tyvar.fresh();
        new_tyvars_for_gen.push(new_tyvar.clone());
        sigma_m.insert(alpha_i_name, new_tyvar);
    }
    let rslt = Type::Poly {
        tyvars: new_tyvars_for_gen,
        poly_type: Box::new(ungeneralizeded_type.clone()),
    };
    rslt
}

pub fn instantiate(
    sigma_m: &mut HashMap<String, Type>,
    gen_fresh_tyvar: &mut FreshTyvarGenerator,
    gen_fresh_meta: &mut FreshMetaGenerator,
    poly: &Type,
) -> Type {
    match poly {
        Type::Poly { tyvars, poly_type } => {
            let mut tyvar_to_concrete_meta: HashMap<String, Type> = HashMap::new();
            for a_i in tyvars.iter() {
                let a_i_name = match a_i {
                    Type::Tyvar { name } => name.clone(),
                    _ => panic!(),
                };
                tyvar_to_concrete_meta.insert(a_i_name, gen_fresh_meta.fresh());
            }
            subst(gen_fresh_tyvar, poly_type, &tyvar_to_concrete_meta, sigma_m)
        }
        _ => poly.clone(),
    }
}

pub fn unify(
    sigma_m: &mut HashMap<String, Type>,
    gen_fresh_meta: &mut FreshMetaGenerator,
    gen_fresh_tyvar: &mut FreshTyvarGenerator,
    ty1: &Type,
    ty2: &Type,
) -> Result<(), String> {
    match (ty1, ty2) {
        (Type::Int, Type::Int)
        | (Type::Bool, Type::Bool)
        | (Type::Unit, Type::Unit)
        | (Type::Action, Type::Action) => Ok(()),
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
            let mut tyvars2_to_tyvars1: HashMap<String, Type> = HashMap::new();
            for (i, j) in iter::zip(alphas1.iter(), alphas2.iter()).into_iter() {
                let name = match i {
                    Type::Tyvar { name: s } => s.clone(),
                    _ => return Err(format!("non tyvar within polytype args")),
                };
                tyvars2_to_tyvars1.insert(name, j.clone());
            }
            let substed_u2 = subst(gen_fresh_tyvar, u2.deref(), &tyvars2_to_tyvars1, sigma_m);
            unify(sigma_m, gen_fresh_meta, gen_fresh_tyvar, u1, &substed_u2)
        }
        (Type::Tyvar { name: name1 }, Type::Tyvar { name: name2 }) => {
            if name1 == name2 {
                Ok(())
            } else {
                Err(format!("unifying two different tyvars"))
            }
        }
        (Type::Meta { name: alpha }, t) => {
            if sigma_m.contains_key(alpha) {
                let sigma_m_alpha = sigma_m.get(alpha).expect("").clone();
                unify(sigma_m, gen_fresh_meta, gen_fresh_tyvar, &sigma_m_alpha, t)
            } else if let Type::Meta { name: gamma } = t {
                if sigma_m.contains_key(gamma) {
                    let sigma_m_gamma = sigma_m.get(gamma).expect("").clone();
                    unify(
                        sigma_m,
                        gen_fresh_meta,
                        gen_fresh_tyvar,
                        &Type::Meta {
                            name: alpha.clone(),
                        },
                        &sigma_m_gamma,
                    )
                } else if gamma == alpha {
                    Ok(())
                } else {
                    sigma_m.insert(alpha.clone(), t.clone());
                    Ok(())
                }
            } else if all_metas_in_type(t).contains(ty1) {
                Err(format!("ty1==Meta(alpha) in t, while t!=Meta(alpha)"))
            } else {
                sigma_m.insert(alpha.clone(), t.clone());
                Ok(())
            }
        }
        (t, Type::Meta { name: alpha }) => unify(
            sigma_m,
            gen_fresh_meta,
            gen_fresh_tyvar,
            &Type::Meta {
                name: alpha.clone(),
            },
            t,
        ),
        _ => Err(format!(
            "{color_red}unification error, {:?} and {:?} incompatible{color_reset}",
            ty1, ty2
        )),
    }
}

pub fn id_is_used_in_expr(id: &str, expr: &meerast::Expr) -> bool {
    match expr {
        meerast::Expr::IdExpr { ident: name } => name == id,
        meerast::Expr::IntConst { val: _ } | meerast::Expr::BoolConst { val: _ } => false,
        meerast::Expr::Action { stmt: _ } => false,
        meerast::Expr::Member {
            srv_name: _,
            member: _,
        } => todo!(),
        meerast::Expr::Apply { fun, args } => {
            let mut rslt = false;
            if match fun.deref() {
                meerast::Expr::IdExpr { ident } => ident.clone(),
                _ => "".to_string(),
            } == id
            {
                rslt = true;
            }
            for i in args.iter() {
                match i {
                    meerast::Expr::IdExpr { ident: arg_name } => {
                        if id == arg_name {
                            rslt = true
                        }
                    }
                    _ => {}
                }
            }
            rslt
        }
        meerast::Expr::BopExpr { opd1, opd2, bop: _ } => {
            id_is_used_in_expr(id, opd1) || id_is_used_in_expr(id, opd2)
        }
        meerast::Expr::UopExpr { opd, uop: _ } => id_is_used_in_expr(id, opd),
        meerast::Expr::IfExpr { cond, then, elze } => {
            id_is_used_in_expr(id, cond)
                || id_is_used_in_expr(id, then)
                || id_is_used_in_expr(id, elze)
        }
        meerast::Expr::Lambda { pars: _, body: _ } => false,
    }
}

pub fn check_decl(
    sigma_v: &mut HashMap<String, Type>,
    sigma_m: &mut HashMap<String, Type>,
    pub_access: &mut HashMap<String, bool>,
    gen_fresh_meta: &mut FreshMetaGenerator,
    gen_fresh_tyvar: &mut FreshTyvarGenerator,
    decl: &meerast::Decl,
) -> Result<(), String> {
    match decl {
        meerast::Decl::Import { srv_name: _ } => Ok(()),
        meerast::Decl::VarDecl { name, val } => {
            let src_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, val);
            let src_type = match src_type {
                Ok(ty) => ty,
                Err(err_msg) => return Err(err_msg),
            };
            let t1 = if id_is_used_in_expr(name, val) {
                Type::Poly {
                    tyvars: vec![],
                    poly_type: Box::new(src_type),
                }
            } else {
                generalize(sigma_m, sigma_v, &src_type, gen_fresh_tyvar)
            };
            sigma_v.insert(name.clone(), t1);
            Ok(())
        }
        meerast::Decl::DefDecl { name, val, is_pub } => {
            let src_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, val);
            let src_type = match src_type {
                Ok(ty) => ty,
                Err(err_msg) => return Err(err_msg),
            };
            let t1 = if id_is_used_in_expr(name, val) {
                Type::Poly {
                    tyvars: vec![],
                    poly_type: Box::new(src_type),
                }
            } else {
                generalize(sigma_m, sigma_v, &src_type, gen_fresh_tyvar)
            };
            pub_access.insert(name.clone(), is_pub.clone());
            sigma_v.insert(name.clone(), t1);
            Ok(())
        }
    }
}

pub fn check_expr(
    sigma_v: &HashMap<String, Type>,
    sigma_m: &mut HashMap<String, Type>,
    gen_fresh_meta: &mut FreshMetaGenerator,
    gen_fresh_tyvar: &mut FreshTyvarGenerator,
    expr: &meerast::Expr,
) -> Result<Type, String> {
    match expr {
        meerast::Expr::IdExpr { ident } => {
            let ident_val_in_sigma_v = sigma_v.get(ident).expect("");
            let rslt = instantiate(
                sigma_m,
                gen_fresh_tyvar,
                gen_fresh_meta,
                ident_val_in_sigma_v,
            );
            Ok(rslt)
        }
        meerast::Expr::IntConst { val: _ } => Ok(Type::Int),
        meerast::Expr::BoolConst { val: _ } => Ok(Type::Bool),
        meerast::Expr::Action { stmt } => {
            let sgls = match stmt {
                meerast::Stmt::Stmt { sgl_stmts } => sgl_stmts,
            };
            for sgl in sgls.iter() {
                match sgl {
                    meerast::SglStmt::Do { act } => {
                        let check_act_result =
                            check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, act);
                        match check_act_result {
                            Ok(_) => {}
                            Err(err_msg) => return Err(err_msg),
                        }
                    }
                    meerast::SglStmt::Ass { dst, src } => {
                        let dst_type = match check_expr(
                            sigma_v,
                            sigma_m,
                            gen_fresh_meta,
                            gen_fresh_tyvar,
                            dst,
                        ) {
                            Ok(ty) => ty,
                            Err(err_msg) => return Err(err_msg),
                        };
                        let src_type: Type =
                            check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, src)?;
                        let _ = unify(
                            sigma_m,
                            gen_fresh_meta,
                            gen_fresh_tyvar,
                            &dst_type,
                            &src_type,
                        )?;
                    }
                }
            }
            Ok(Type::Action)
        }
        meerast::Expr::Member {
            srv_name: _,
            member: _,
        } => todo!(),
        meerast::Expr::Apply { fun, args } => {
            let fun_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, fun)?;
            let mut arg_types: Vec<Type> = vec![];
            for arg in args.iter() {
                let arg_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, arg);
                let arg_type = match arg_type {
                    Ok(ty) => ty,
                    Err(err_msg) => return Err(err_msg),
                };
                arg_types.push(arg_type);
            }
            let ret_type = gen_fresh_meta.fresh();
            let _ = unify(
                sigma_m,
                gen_fresh_meta,
                gen_fresh_tyvar,
                &fun_type,
                &Type::Fun {
                    par_types: arg_types,
                    ret_type: Box::new(ret_type.clone()),
                },
            )?;
            Ok(ret_type)
        }
        meerast::Expr::BopExpr { opd1, opd2, bop } => match bop {
            meerast::Binop::Add
            | meerast::Binop::Sub
            | meerast::Binop::Mul
            | meerast::Binop::Div => {
                let opd1_type =
                    check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, opd1)?;
                let opd2_type =
                    check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, opd2)?;
                let _ = unify(
                    sigma_m,
                    gen_fresh_meta,
                    gen_fresh_tyvar,
                    &opd1_type,
                    &Type::Int,
                )?;
                let _ = unify(
                    sigma_m,
                    gen_fresh_meta,
                    gen_fresh_tyvar,
                    &opd2_type,
                    &Type::Int,
                )?;
                Ok(Type::Int)
            }
            meerast::Binop::Eq | meerast::Binop::Lt | meerast::Binop::Gt => {
                let opd1_type =
                    check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, opd1)?;
                let opd2_type =
                    check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, opd2)?;
                let _ = unify(
                    sigma_m,
                    gen_fresh_meta,
                    gen_fresh_tyvar,
                    &opd1_type,
                    &Type::Int,
                )?;
                let _ = unify(
                    sigma_m,
                    gen_fresh_meta,
                    gen_fresh_tyvar,
                    &opd2_type,
                    &Type::Int,
                )?;
                Ok(Type::Bool)
            }
            meerast::Binop::And | meerast::Binop::Or => {
                let opd1_type =
                    check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, opd1)?;
                let opd2_type =
                    check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, opd2)?;
                let _ = unify(
                    sigma_m,
                    gen_fresh_meta,
                    gen_fresh_tyvar,
                    &opd1_type,
                    &Type::Bool,
                )?;
                let _ = unify(
                    sigma_m,
                    gen_fresh_meta,
                    gen_fresh_tyvar,
                    &opd2_type,
                    &Type::Bool,
                )?;
                Ok(Type::Bool)
            }
        },
        meerast::Expr::UopExpr { opd, uop } => match uop {
            meerast::Uop::Neg => {
                let opd_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, opd)?;
                let _ = unify(
                    sigma_m,
                    gen_fresh_meta,
                    gen_fresh_tyvar,
                    &opd_type,
                    &Type::Int,
                )?;
                Ok(Type::Int)
            }
            meerast::Uop::Not => {
                let opd_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, opd)?;
                let _ = unify(
                    sigma_m,
                    gen_fresh_meta,
                    gen_fresh_tyvar,
                    &opd_type,
                    &Type::Bool,
                )?;
                Ok(Type::Bool)
            }
        },
        meerast::Expr::IfExpr { cond, then, elze } => {
            let cond_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, cond)?;
            let then_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, then)?;
            let elze_type = check_expr(sigma_v, sigma_m, gen_fresh_meta, gen_fresh_tyvar, elze)?;
            let _ = unify(
                sigma_m,
                gen_fresh_meta,
                gen_fresh_tyvar,
                &cond_type,
                &Type::Bool,
            )?;
            let _ = unify(
                sigma_m,
                gen_fresh_meta,
                gen_fresh_tyvar,
                &then_type,
                &elze_type,
            )?;
            Ok(then_type)
        }
        meerast::Expr::Lambda { pars, body } => {
            let mut new_metas_for_pars: Vec<Type> = vec![];
            let mut par_to_type: HashMap<String, Type> = HashMap::new();
            for x in pars.iter() {
                let fresh_meta = gen_fresh_meta.fresh();
                new_metas_for_pars.push(fresh_meta.clone());
                let x_name = match x {
                    meerast::Expr::IdExpr { ident } => ident.clone(),
                    _ => panic!(),
                };
                par_to_type.insert(x_name, fresh_meta);
            }
            let t2 = gen_fresh_meta.fresh();
            let mut local_sigma_v = sigma_v.clone();
            local_sigma_v.extend(par_to_type.into_iter());
            let t3 = check_expr(
                &local_sigma_v,
                sigma_m,
                gen_fresh_meta,
                gen_fresh_tyvar,
                body,
            )?;
            let _ = unify(sigma_m, gen_fresh_meta, gen_fresh_tyvar, &t2, &t3)?;
            Ok(Type::Fun {
                par_types: new_metas_for_pars,
                ret_type: Box::new(t2),
            })
        }
    }
}

pub fn check_prog_test(srv: &meerast::Service) {
    let mut gen_fresh_meta = FreshMetaGenerator::new("default", 0);
    let mut gen_fresh_tyvar = FreshTyvarGenerator::new("default", 0);
    let mut sigma_v: HashMap<String, Type> = HashMap::new();
    let mut sigma_m: HashMap<String, Type> = HashMap::new();
    let mut pub_access: HashMap<String, bool> = HashMap::new();
    match srv {
        meerast::Service::Srv { name: _, decls } => {
            for decl in decls.iter() {
                let check_rslt = check_decl(
                    &mut sigma_v,
                    &mut sigma_m,
                    &mut pub_access,
                    &mut gen_fresh_meta,
                    &mut gen_fresh_tyvar,
                    decl,
                );
                match check_rslt {
                    Ok(_) => {}
                    Err(_) => panic!(),
                }
            }
        }
    }
    println!("sigma_m:\n{:?}", sigma_m);
    println!("sigma_v:\n{:?}", sigma_v);
}
