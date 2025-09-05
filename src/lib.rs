mod return_type;
mod utils;
mod visitors;

use std::ops::DerefMut;

use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    Expr, ExprPath, Ident, ItemFn, MetaNameValue, Token, parse::Parse, punctuated::Punctuated,
    visit_mut::VisitMut,
};

/// # Example
/// ```rust
/// #[fnerror]
/// fn foo() -> Result<()> {
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
/// #[derive(Debug, ::thiserror::Error)]
/// pub enum FooError {
///     #[error("{}", 0usize)]
///     Error2(String),
///     #[error("{}, {}",0usize, 1usize)]
///     Error3(&'static str, u8),
/// }
/// fn foo() -> ::std::result::Result<(), FooError> {
///     bar().map_err(|e| FooError::Error2(e))?;
///     baz().map_err(|e| FooError::Error3(e, 123))?;
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn fnerror(args: TokenStream, item: TokenStream) -> TokenStream {
    let item = TokenStream2::from(item);
    let args = TokenStream2::from(args);
    let mut function: ItemFn = syn::parse2(item).expect("expect a function");

    let fn_ident = &function.sig.ident;

    let args: Args = syn::parse2(args).expect("unknown attribute args");

    let error_ident = args.name.unwrap_or_else(|| {
        Ident::new(
            &format!("{}Error", fn_ident.to_string().to_pascal_case()),
            Span::call_site(),
        )
    });
    let block = &mut function.block;

    let mut visitor = visitors::FnerrVistor::new(error_ident.clone(), &function.sig.generics);
    visitor.visit_block_mut(block.deref_mut());
    let error_item = visitor.error_item;

    return_type::parse_return_type(
        utils::path_from_args(error_ident, visitor.generic_args),
        &mut function.sig.output,
    );

    quote! {
        #error_item
        #function
    }
    .into()
}

#[derive(Default)]
struct Args {
    name: Option<Ident>,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let meta_list = Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?;
        let args = meta_list.iter().fold(Self::default(), |mut acc, cur| {
            if cur.path.is_ident("ident")
                && let Expr::Path(ExprPath { path, .. }) = &cur.value
            {
                acc.name = path.get_ident().cloned();
            }
            acc
        });
        Ok(args)
    }
}
