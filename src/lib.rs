use std::ops::DerefMut;

use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    AngleBracketedGenericArguments, AttrStyle, Attribute, Expr, ExprCall, ExprCast, Field,
    FieldMutability, Fields, FieldsUnnamed, Generics, Ident, ItemEnum, ItemFn, MacroDelimiter,
    Meta, MetaList, Path, PathArguments, PathSegment, ReturnType, Token, Type, TypePath, Variant,
    Visibility,
    punctuated::Punctuated,
    token::{Brace, Enum},
    visit_mut::{self, VisitMut},
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
///     #[error("{}, {}",0usize 1usize)]
///     Error3(&'static str, u8),
/// }
/// fn foo() -> ::std::result::Result<(), FooError> {
///     bar().map_err(|e| FooError::Error2(e))?;
///     baz().map_err(|e| FooError::Error3(e, 123))?;
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn fnerror(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item = TokenStream2::from(item);
    let mut function: ItemFn = syn::parse2(item).unwrap();

    let fn_ident = &function.sig.ident;
    let error_ident = Ident::new(
        &format!("{}Error", fn_ident.to_string().to_pascal_case()),
        Span::call_site(),
    );

    parse_return_type(error_ident.clone(), &mut function.sig.output);

    let block = &mut function.block;

    let mut visitor = ErrorVistor::new(error_ident);
    visitor.visit_block_mut(block.deref_mut());
    let error_item = visitor.error;

    quote! {
        #error_item
        #function
    }
    .into()
}

fn parse_return_type(error_ident: Ident, return_ty: &mut ReturnType) {
    if let ReturnType::Type(_, ty) = return_ty
        && let Type::Path(TypePath {
            path: Path {
                leading_colon,
                segments,
            },
            ..
        }) = ty.as_mut()
    {
        *leading_colon = Some(Default::default());

        let segment = segments
            .first_mut()
            .filter(|segment| segment.ident == "Result")
            .expect("expect `Result<T>`");

        if let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
            &mut segment.arguments
        {
            if args.len() != 1 {
                panic!("expect `Result<T>`, fnerror will generate a error for this");
            }
            args.push(syn::GenericArgument::Type(Type::Path(TypePath {
                qself: None,
                path: path_from_ident(error_ident.clone()),
            })));
        }

        let mut new_segments = Punctuated::<_, Token![::]>::new();

        new_segments.push(PathSegment {
            ident: call_site_ident("std"),
            arguments: Default::default(),
        });
        new_segments.push(PathSegment {
            ident: call_site_ident("result"),
            arguments: Default::default(),
        });

        new_segments.push(segment.clone());

        *segments = new_segments;
    }
}

struct ErrorVistor {
    error: ItemEnum,
}

impl ErrorVistor {
    fn new(error_ident: Ident) -> Self {
        let error = ItemEnum {
            attrs: vec![Attribute {
                pound_token: Default::default(),
                style: AttrStyle::Outer,
                bracket_token: Default::default(),
                meta: Meta::List(MetaList {
                    path: path_from_str("derive"),
                    delimiter: MacroDelimiter::Paren(Default::default()),
                    tokens: quote! { Debug, ::thiserror::Error },
                }),
            }],
            vis: Visibility::Public(Default::default()),
            enum_token: Enum::default(),
            ident: error_ident,
            generics: Generics::default(),
            brace_token: Brace::default(),
            variants: Punctuated::new(),
        };
        Self { error }
    }
}

impl VisitMut for ErrorVistor {
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
                ident: self.error.ident.clone(),
                arguments: PathArguments::None,
            },
        );

        let mut args = call.args.iter();

        let fmt = args.next().expect("expect a format string");

        let (new_args, fields): (Punctuated<_, Token![,]>, Punctuated<_, Token![,]>) = args
            .map(|arg| {
                let Expr::Cast(ExprCast { expr, ty, .. }) = arg else {
                    panic!("expect a PatType, like: `context: &'static str`");
                };
                let ty = ty.as_ref().clone();
                let pat = expr.as_ref().clone();

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

        self.error.variants.push(Variant {
            attrs: vec![Attribute {
                pound_token: Default::default(),
                style: AttrStyle::Outer,
                bracket_token: Default::default(),
                meta: Meta::List(MetaList {
                    path: path_from_str("error"),
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

fn path_from_str(ident: &str) -> Path {
    path_from_ident(call_site_ident(ident))
}

fn path_from_ident(ident: Ident) -> Path {
    Path::from(PathSegment::from(ident))
}

fn call_site_ident(ident: &str) -> Ident {
    Ident::new(ident, Span::call_site())
}
