use std::ops::DerefMut;

use syn::{
    AttrStyle, Attribute, Expr, ExprCall, ExprCast, Field, FieldMutability, Fields, FieldsUnnamed,
    GenericArgument, Generics, Ident, ItemEnum, MacroDelimiter, Meta, MetaList, PathArguments,
    PathSegment, Token, Variant, Visibility,
    punctuated::Punctuated,
    visit::Visit,
    visit_mut::{self, VisitMut},
};

use crate::{utils, visitors::generics::GenericsVisitor};

use quote::quote;

pub struct FnerrVistor<'a> {
    pub declared_generics: &'a Generics,
    pub error_item: ItemEnum,
    pub generic_args: Punctuated<GenericArgument, Token![,]>,
}

impl<'a> FnerrVistor<'a> {
    pub(crate) fn new(error_ident: Ident, declared_generics: &'a Generics) -> Self {
        let error = ItemEnum {
            attrs: vec![Attribute {
                pound_token: Default::default(),
                style: AttrStyle::Outer,
                bracket_token: Default::default(),
                meta: Meta::List(MetaList {
                    path: utils::path_from_str("derive"),
                    delimiter: MacroDelimiter::Paren(Default::default()),
                    tokens: quote! { Debug, ::thiserror::Error },
                }),
            }],
            vis: Visibility::Public(Default::default()),
            enum_token: Default::default(),
            ident: error_ident,
            generics: Generics::default(),
            brace_token: Default::default(),
            variants: Punctuated::new(),
        };
        Self {
            declared_generics,
            error_item: error,
            generic_args: Punctuated::new(),
        }
    }
}

impl<'a> VisitMut for FnerrVistor<'a> {
    fn visit_expr_call_mut(&mut self, call: &mut ExprCall) {
        let mut no_fnerr = true;

        call.attrs.retain(|attr| {
            attr.meta.path().get_ident().is_none_or(|ident| {
                if ident == "fnerr" {
                    // find a #[fnerr]
                    no_fnerr = false;
                    // don't retain #[fnerr]
                    false
                } else {
                    // retain other attribute
                    true
                }
            })
        });
        if no_fnerr {
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
                ident: self.error_item.ident.clone(),
                arguments: PathArguments::None,
            },
        );

        let mut generics_visitor = GenericsVisitor::new(
            self.declared_generics,
            &mut self.error_item.generics,
            &mut self.generic_args,
        );

        let mut args = call.args.iter();

        let fmt = args.next().expect("expect a format string");

        let (new_args, fields): (Punctuated<_, Token![,]>, Punctuated<_, Token![,]>) = args
            .map(|arg| {
                let Expr::Cast(ExprCast { expr, ty, .. }) = arg else {
                    panic!("expect a cast expression, like: `context as &'static str`");
                };

                generics_visitor.visit_type(ty);

                let pat = expr.as_ref().clone();
                let ty = ty.as_ref().clone();

                let field = Field {
                    attrs: Vec::new(),
                    vis: Visibility::Inherited,
                    mutability: FieldMutability::None,
                    ident: None,
                    colon_token: None,
                    ty,
                };
                (pat, field)
            })
            .unzip();

        let fields_count = fields.len();
        let fields_index = 0..fields_count;

        self.error_item.variants.push(Variant {
            attrs: vec![Attribute {
                pound_token: Default::default(),
                style: AttrStyle::Outer,
                bracket_token: Default::default(),
                meta: Meta::List(MetaList {
                    path: utils::path_from_str("error"),
                    delimiter: MacroDelimiter::Paren(Default::default()),
                    tokens: quote! { #fmt, #(#fields_index),* },
                }),
            }],
            ident,
            fields: Fields::Unnamed(FieldsUnnamed {
                unnamed: fields,
                paren_token: Default::default(),
            }),
            discriminant: None,
        });

        call.args = new_args;
    }
}
