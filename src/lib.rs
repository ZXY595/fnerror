mod return_type;
mod visitors;

use std::collections::VecDeque;

use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Block, FnArg, GenericArgument, GenericParam, Generics, Ident, Token, Visibility,
    parse::Parse, parse_quote, punctuated::Punctuated, spanned::Spanned, token,
    visit_mut::VisitMut,
};

use crate::{
    return_type::{GenericErrType, ReturnType},
    visitors::{FnErrErrorMeta, FnErrExprVistor},
};

/// # Example
/// ```rust
/// #[fnerror]
/// fn foo() -> Result<(), MyError> {
///     bar().map_err(|e| {
///         #[fnerr]
///         Error2("{}", e as String)
///     })?;
///     baz().map_err(|e| {
///         #[fnerr]
///         Error3("{}, {}", e as &'static str, 123 as u8)
///     })?;
///     Ok(())
/// }
///
/// fn bar() -> Result<(), String> {
///     Err("test2 error".to_string())
/// }
///
/// fn baz() -> Result<(), &'static str> {
///     Err("test2 error")
/// }
/// ```
///
/// will be expanded to if thiserror feature is enabled:
///
/// ```rust
/// fn foo() -> ::std::result::Result<(), MyError> {
///     bar().map_err(|e| MyError::Error2(e))?;
///     baz().map_err(|e| MyError::Error3(e, 123))?;
///     Ok(())
/// }
/// #[derive(Debug, ::thiserror::Error)]
/// pub enum MyError {
///     #[error("{}", 0usize)]
///     Error2(String),
///     #[error("{}, {}",0usize, 1usize)]
///     Error3(&'static str, u8),
/// }
/// ```
#[proc_macro_attribute]
pub fn fnerror(_args: TokenStream, item: TokenStream) -> TokenStream {
    syn::parse_macro_input!(item as ItemFn)
        .to_token_stream()
        .into()
}

struct ItemFn {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
    pub err_ty_ident: Ident,
    pub fnerr_generics: VecDeque<GenericParam>,
    pub block: Box<Block>,
    pub fnerr_meta: Vec<FnErrErrorMeta>,
}

impl Parse for ItemFn {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let sig: Signature = input.parse()?;
        let mut block: Box<Block> = input.parse()?;

        let err_ty_ident = sig.output.err_type.clone().unwrap_or_else(|| {
            Ident::new(
                &format!("{}Error", sig.ident.to_string().to_pascal_case()),
                Span::call_site(),
            )
        });

        let mut fnerr_meta = vec![];
        let mut found_generics = VecDeque::new();
        let mut visitor = FnErrExprVistor::new(
            err_ty_ident.clone(),
            &sig.generics,
            &mut fnerr_meta,
            &mut found_generics,
        );
        visitor.visit_block_mut(&mut block);

        Ok(Self {
            attrs,
            vis,
            sig,
            err_ty_ident,
            fnerr_generics: found_generics,
            block,
            fnerr_meta,
        })
    }
}

impl ToTokens for ItemFn {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.attrs.iter().for_each(|attr| attr.to_tokens(tokens));
        self.vis.to_tokens(tokens);

        let fnerr_generics_args = self
            .fnerr_generics
            .iter()
            .cloned()
            .map(Generic)
            .map(Generic::<GenericArgument>::from)
            .map(|generic| generic.0)
            .collect();
        let err_ty_ident = &self.err_ty_ident;

        Printer::new(
            &self.sig,
            &GenericErrType {
                ident: err_ty_ident.clone(),
                generics: fnerr_generics_args,
            },
        )
        .to_tokens(tokens);

        self.block.to_tokens(tokens);

        let fnerr_generics = self.fnerr_generics.iter();
        let fnerr_meta = &self.fnerr_meta;

        tokens.extend(quote! {
            #[derive(Debug, ::thiserror::Error)]
            pub enum #err_ty_ident<#(#fnerr_generics),*> {
                #(#fnerr_meta),*
            }
        });
    }
}

struct Signature {
    pub constness: Option<Token![const]>,
    pub asyncness: Option<Token![async]>,
    pub unsafety: Option<Token![unsafe]>,
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub generics: Generics,
    pub paren_token: token::Paren,
    pub inputs: Punctuated<FnArg, Token![,]>,
    pub output: ReturnType,
}

impl Parse for Signature {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            constness: input.parse()?,
            asyncness: input.parse()?,
            unsafety: input.parse()?,
            fn_token: input.parse()?,
            ident: input.parse()?,
            generics: input.parse()?,
            paren_token: syn::parenthesized!(content in input),
            inputs: content.parse_terminated(FnArg::parse, Token![,])?,
            output: input.parse()?,
        })
    }
}

impl ToTokens for Printer<&Signature, &GenericErrType> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let sig = &self.inner;
        sig.constness.to_tokens(tokens);
        sig.asyncness.to_tokens(tokens);
        sig.unsafety.to_tokens(tokens);
        sig.fn_token.to_tokens(tokens);
        sig.ident.to_tokens(tokens);
        sig.generics.to_tokens(tokens);
        sig.paren_token.surround(tokens, |tokens| {
            sig.inputs.to_tokens(tokens);
        });
        Printer::new(&sig.output, self.meta).to_tokens(tokens);
    }
}

/// Helper struct for types that needs meta to be printed in tokens.
struct Printer<T, M> {
    inner: T,
    meta: M,
}

impl<T, M> Printer<T, M> {
    fn new(inner: T, meta: M) -> Self {
        Self { inner, meta }
    }
}

struct Generic<T>(pub T);

impl TryFrom<Generic<GenericArgument>> for Generic<GenericParam> {
    type Error = syn::Error;
    fn try_from(Generic(argument): Generic<GenericArgument>) -> Result<Self, Self::Error> {
        let param = match argument {
            GenericArgument::Lifetime(lifetime) => GenericParam::Lifetime(parse_quote!(#lifetime)),
            GenericArgument::Type(ty) => GenericParam::Type(parse_quote!(#ty)),
            GenericArgument::Const(expr) => GenericParam::Const(parse_quote!(#expr)),
            argument => {
                return Err(syn::Error::new(
                    argument.span(),
                    "unsupported generic argument",
                ));
            }
        };
        Ok(Self(param))
    }
}

impl From<Generic<GenericParam>> for Generic<GenericArgument> {
    fn from(Generic(param): Generic<GenericParam>) -> Self {
        let argument = match param {
            GenericParam::Lifetime(param) => GenericArgument::Lifetime(param.lifetime),
            GenericParam::Type(param) => GenericArgument::Type({
                let ident = param.ident;
                parse_quote!(#ident)
            }),
            GenericParam::Const(param) => GenericArgument::Const({
                let ident = param.ident;
                parse_quote!(#ident)
            }),
        };
        Self(argument)
    }
}
