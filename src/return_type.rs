use quote::{ToTokens, quote};
use syn::{Error, GenericArgument, Ident, Token, Type, parse::Parse};

use crate::Printer;

pub struct ReturnType {
    pub arrow: Token![->],
    pub ident: Ident,
    pub lt_token: Token![<],
    pub ok_type: Type,
    pub comma: Option<Token![,]>,
    pub err_type: Option<Ident>,
    pub gt_token: Token![>],
}

impl Parse for ReturnType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            arrow: input.parse()?,
            ident: input.parse().and_then(|ident| {
                (ident == "Result")
                    .then_some(ident)
                    .ok_or(Error::new(input.span(), "expect Result"))
            })?,
            lt_token: input.parse()?,
            ok_type: input.parse()?,
            comma: input.parse()?,
            err_type: input.parse()?,
            gt_token: input.parse()?,
        })
    }
}

pub struct GenericErrType {
    pub ident: Ident,
    pub generics: Vec<GenericArgument>,
}

impl ToTokens for GenericErrType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = &self.ident;
        let generics = &self.generics;
        tokens.extend(quote! {
            #ident<#(#generics),*>
        })
    }
}

impl ToTokens for Printer<&ReturnType, &GenericErrType> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let return_type = &self.inner;
        return_type.arrow.to_tokens(tokens);
        return_type.ident.to_tokens(tokens);
        return_type.lt_token.to_tokens(tokens);
        return_type.ok_type.to_tokens(tokens);
        return_type.comma.unwrap_or_default().to_tokens(tokens);
        self.meta.to_tokens(tokens);
        return_type.gt_token.to_tokens(tokens);
    }
}
