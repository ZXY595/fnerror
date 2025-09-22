use std::collections::VecDeque;

use syn::{
    Expr, ExprPath, GenericArgument, GenericParam, Generics, LifetimeParam, TypePath,
    TypeReference,
    punctuated::Punctuated,
    visit::{self, Visit},
};

pub struct GenericsVisitor<'a> {
    pub declared_generics: &'a Generics,
    pub found_generics: &'a mut VecDeque<GenericParam>,
}

impl<'a> GenericsVisitor<'a> {}

impl<'a> GenericsVisitor<'a> {
    pub fn new(
        declared_generics: &'a Generics,
        found_generics: &'a mut VecDeque<GenericParam>,
    ) -> Self {
        Self {
            declared_generics,
            found_generics,
        }
    }
}

impl<'ast> Visit<'ast> for GenericsVisitor<'ast> {
    fn visit_type_reference(&mut self, i: &'ast TypeReference) {
        let TypeReference { lifetime, elem, .. } = i;

        let lifetime = lifetime.clone().expect("expect a lifetime");
        if let Some(param) = self
            .declared_generics
            .lifetimes()
            .find(|param| param.lifetime == lifetime)
            && self
                .found_generics
                .iter()
                .filter_map(|param| {
                    if let GenericParam::Lifetime(param) = param {
                        Some(param)
                    } else {
                        None
                    }
                })
                .all(|founded_param| founded_param.lifetime != param.lifetime)
        {
            let param = GenericParam::Lifetime(param.clone());
            self.found_generics.push_front(param);

            visit::visit_type(self, elem);
        } else {
            // not declared in the function generics, not a generics
            visit::visit_type_reference(self, i)
        };
    }

    fn visit_type_path(&mut self, i: &'ast TypePath) {
        let TypePath { path, .. } = i;
        let ident = path
            .segments
            .last()
            .expect("expect a path segment, like: `std::vec::Vec<T>`")
            .ident
            .clone();

        if let Some(param) = self
            .declared_generics
            .type_params()
            .find(|param| param.ident == ident)
            && self
                .found_generics
                .iter()
                .filter_map(|param| {
                    if let GenericParam::Type(param) = param {
                        Some(param)
                    } else {
                        None
                    }
                })
                .all(|founded_param| founded_param.ident != param.ident)
        {
            let param = GenericParam::Type(param.clone());

            self.found_generics.push_back(param);
        } else {
            // not declared in the function generics, not a generics
            visit::visit_type_path(self, i)
        };
    }

    fn visit_expr_path(&mut self, i: &'ast ExprPath) {
        let ExprPath { path, .. } = i;
        if let Some(ident) = path.get_ident()
            && let Some(param) = self
                .declared_generics
                .const_params()
                .find(|param| param.ident == ident.clone())
            && self
                .found_generics
                .iter()
                .filter_map(|param| {
                    if let GenericParam::Const(param) = param {
                        Some(param)
                    } else {
                        None
                    }
                })
                .all(|founded_param| founded_param.ident != param.ident)
        {
            self.found_generics
                .push_back(GenericParam::Const(param.clone()));
        } else {
            visit::visit_expr_path(self, i)
        }
    }

    fn visit_generic_argument(&mut self, i: &'ast GenericArgument) {
        match i {
            GenericArgument::Lifetime(lifetime)
                if self
                    .found_generics
                    .iter()
                    .filter_map(|param| {
                        if let GenericParam::Lifetime(param) = param {
                            Some(param)
                        } else {
                            None
                        }
                    })
                    .all(|founded_param| &founded_param.lifetime != lifetime) =>
            {
                let param = GenericParam::Lifetime(LifetimeParam {
                    attrs: Vec::new(),
                    lifetime: lifetime.clone(),
                    colon_token: None,
                    bounds: Punctuated::new(),
                });
                self.found_generics.push_front(param);
            }
            GenericArgument::Const(Expr::Path(ExprPath { path, .. })) => {
                let ident = path
                    .segments
                    .last()
                    .expect("expect a path segment, like: `std::vec::Vec<T>`")
                    .ident
                    .clone();
                if let Some(param) = self
                    .declared_generics
                    .const_params()
                    .find(|param| param.ident == ident)
                    && self
                        .found_generics
                        .iter()
                        .filter_map(|param| {
                            if let GenericParam::Const(param) = param {
                                Some(param)
                            } else {
                                None
                            }
                        })
                        .all(|founded_param| founded_param.ident != ident)
                {
                    let param = GenericParam::Const(param.clone());

                    self.found_generics.push_back(param);
                } else {
                    // not declared in the function generics, not a generics
                    visit::visit_generic_argument(self, i)
                };
            }
            _ => visit::visit_generic_argument(self, i),
        }
    }
}
