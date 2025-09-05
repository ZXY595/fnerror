use syn::{
    Expr, ExprPath, GenericArgument, GenericParam, Generics, LifetimeParam, Token, Type, TypePath,
    TypeReference,
    punctuated::Punctuated,
    visit::{self, Visit},
};

pub struct GenericsVisitor<'a> {
    pub declared_generics: &'a Generics,
    pub generics: &'a mut Generics,
    /// use for return type
    pub generic_args: &'a mut Punctuated<GenericArgument, Token![,]>,
}

impl<'a> GenericsVisitor<'a> {
    pub fn new(
        declared_generics: &'a Generics,
        generics: &'a mut Generics,
        args: &'a mut Punctuated<GenericArgument, Token![,]>,
    ) -> Self {
        Self {
            declared_generics,
            generics,
            generic_args: args,
        }
    }
}

impl<'ast> Visit<'ast> for GenericsVisitor<'ast> {
    fn visit_type_reference(&mut self, i: &'ast TypeReference) {
        let TypeReference { lifetime, elem, .. } = i;

        let lifetime = lifetime.clone().expect("expect a lifetime");
        let Some(param) = self
            .declared_generics
            .lifetimes()
            .find(|param| param.lifetime == lifetime)
        else {
            // not declared in the function generics, not a generics
            return visit::visit_type_reference(self, i);
        };

        visit::visit_type(self, elem);

        self.generic_args
            .push(GenericArgument::Lifetime(lifetime.clone()));

        let param = GenericParam::Lifetime(param.clone());
        self.generics.params.push(param);
    }
    fn visit_type_path(&mut self, i: &'ast TypePath) {
        let TypePath { path, .. } = i;
        let ident = path
            .segments
            .last()
            .expect("expect a path segment, like: `std::vec::Vec<T>`")
            .ident
            .clone();

        let Some(param) = self
            .declared_generics
            .type_params()
            .find(|param| param.ident == ident)
        else {
            // not declared in the function generics, not a generics
            return visit::visit_type_path(self, i);
        };
        let param = GenericParam::Type(param.clone());

        self.generic_args
            .push(GenericArgument::Type(Type::Path(i.clone())));
        self.generics.params.push(param);
    }
    fn visit_expr_path(&mut self, i: &'ast ExprPath) {
        let ExprPath { path, .. } = i;
        if let Some(ident) = path.get_ident()
            && let Some(param) = self
                .declared_generics
                .const_params()
                .find(|param| param.ident == ident.clone())
        {
            self.generics
                .params
                .push(GenericParam::Const(param.clone()));
        } else {
            return visit::visit_expr_path(self, i);
        }

        self.generic_args
            .push(GenericArgument::Type(Type::Path(TypePath {
                qself: None,
                path: path.clone(),
            })));
    }

    fn visit_generic_argument(&mut self, i: &'ast GenericArgument) {
        let param = match i {
            GenericArgument::Lifetime(lifetime) => GenericParam::Lifetime(LifetimeParam {
                attrs: Vec::new(),
                lifetime: lifetime.clone(),
                colon_token: None,
                bounds: Punctuated::new(),
            }),
            GenericArgument::Const(Expr::Path(ExprPath { path, .. })) => {
                let ident = path
                    .segments
                    .last()
                    .expect("expect a path segment, like: `std::vec::Vec<T>`")
                    .ident
                    .clone();
                let Some(param) = self
                    .declared_generics
                    .const_params()
                    .find(|param| param.ident == ident)
                else {
                    // not declared in the function generics, not a generics
                    return visit::visit_generic_argument(self, i);
                };
                GenericParam::Const(param.clone())
            }
            _ => return visit::visit_generic_argument(self, i),
        };
        self.generic_args.push(i.clone());

        self.generics.params.push(param);
    }
}
