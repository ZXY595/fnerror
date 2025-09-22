use std::{collections::VecDeque, ops::DerefMut};

use syn::{
    Expr, ExprCall, ExprCast, GenericParam, Generics, Ident, PathArguments, PathSegment, Token,
    Type,
    punctuated::Punctuated,
    visit::Visit,
    visit_mut::{self, VisitMut},
};

use crate::visitors::generics::GenericsVisitor;

use quote::{ToTokens, quote};

pub struct FnErrExprVistor<'v> {
    err_ty_ident: Ident,
    declared_generics: &'v Generics,
    fnerr_meta: &'v mut Vec<FnErrErrorMeta>,
    found_generics: &'v mut VecDeque<GenericParam>,
}

pub struct FnErrErrorMeta {
    pub ident: Ident,
    pub fmt: Expr,
    pub tys: Punctuated<Type, Token![,]>,
}

impl FnErrErrorMeta {
    fn new(ident: Ident, fmt: Expr, tys: Punctuated<Type, Token![,]>) -> Self {
        Self { ident, fmt, tys }
    }
}

impl ToTokens for FnErrErrorMeta {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let FnErrErrorMeta { ident, fmt, tys } = self;
        let indexs = 0..tys.len();
        tokens.extend(quote! {
            #[error(#fmt, #(#indexs),*)]
            #ident(#tys)
        })
    }
}

impl<'v> FnErrExprVistor<'v> {
    pub(crate) fn new(
        err_ty_ident: Ident,
        declared_generics: &'v Generics,
        fnerr_meta: &'v mut Vec<FnErrErrorMeta>,
        found_generics: &'v mut VecDeque<GenericParam>,
    ) -> Self {
        Self {
            err_ty_ident,
            declared_generics,
            fnerr_meta,
            found_generics,
        }
    }
}

impl VisitMut for FnErrExprVistor<'_> {
    fn visit_item_mut(&mut self, _: &mut syn::Item) {}
    fn visit_expr_call_mut(&mut self, call: &mut ExprCall) {
        let mut fnerr_attrs = call.attrs.extract_if(.., |attr| {
            attr.meta
                .path()
                .get_ident()
                .is_some_and(|ident| ident == "fnerr")
        });

        let no_fnerr_attr = fnerr_attrs.next().is_none();
        drop(fnerr_attrs);

        if no_fnerr_attr {
            return visit_mut::visit_expr_call_mut(self, call);
        }

        let Expr::Path(path) = call.func.deref_mut() else {
            panic!("expect a path")
        };
        let path = &mut path.path;
        let ident = path
            .get_ident()
            .expect("expect a single identifier")
            .clone();

        path.segments.insert(
            0,
            PathSegment {
                ident: self.err_ty_ident.clone(),
                arguments: PathArguments::None,
            },
        );

        let mut generics_visitor =
            GenericsVisitor::new(self.declared_generics, self.found_generics);

        let mut args = call.args.iter();

        let fmt = args.next().cloned().expect("expect a format string");

        let (new_args, tys) = args
            .map(|arg| {
                let Expr::Cast(ExprCast { expr, ty, .. }) = arg else {
                    panic!("expect a cast expression, like: `context as &'static str`");
                };

                generics_visitor.visit_type(ty);

                let pat = expr.as_ref().clone();
                let ty = ty.as_ref().clone();

                (pat, ty)
            })
            .unzip();

        self.fnerr_meta.push(FnErrErrorMeta::new(ident, fmt, tys));
        call.args = new_args;
    }
}
