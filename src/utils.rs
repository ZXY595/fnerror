use proc_macro2::Span;
use syn::{
    AngleBracketedGenericArguments, GenericArgument, Ident, Path, PathArguments, PathSegment,
    Token, punctuated::Punctuated,
};

pub fn path_from_args(ident: Ident, args: Punctuated<GenericArgument, Token![,]>) -> Path {
    let segment = PathSegment {
        ident,
        arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args,
            gt_token: Default::default(),
        }),
    };
    Path::from(segment)
}

pub fn path_from_str(ident: &str) -> Path {
    path_from_ident(call_site_ident(ident))
}

pub fn path_from_ident(ident: Ident) -> Path {
    Path::from(PathSegment::from(ident))
}

pub fn call_site_ident(ident: &str) -> Ident {
    Ident::new(ident, Span::call_site())
}
