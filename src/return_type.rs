use syn::{
    AngleBracketedGenericArguments, GenericArgument, Path, PathArguments, PathSegment, ReturnType,
    Token, Type, TypePath, punctuated::Punctuated,
};

use crate::utils;

pub fn parse_return_type(error_path: Path, return_ty: &mut ReturnType) {
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
            args.push(GenericArgument::Type(Type::Path(TypePath {
                qself: None,
                path: error_path,
            })));
        }

        let mut new_segments = Punctuated::<_, Token![::]>::new();

        new_segments.push(PathSegment {
            ident: utils::call_site_ident("std"),
            arguments: Default::default(),
        });
        new_segments.push(PathSegment {
            ident: utils::call_site_ident("result"),
            arguments: Default::default(),
        });

        new_segments.push(segment.clone());

        *segments = new_segments;
    }
}
