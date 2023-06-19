use syn::{
    AngleBracketedGenericArguments, Data, Fields, FieldsNamed, GenericArgument, Path,
    PathArguments, PathSegment, Type, TypePath,
};

#[allow(dead_code)]
pub fn type_is<T: AsRef<str>>(ty: &Type, name: T) -> bool {
    let name = name.as_ref();
    extract_last_path_segment(ty)
        .map(|path_seg| path_seg.ident == name)
        .unwrap_or(false)
}

#[allow(dead_code)]
pub fn type_is_in<T: AsRef<str>>(ty: &Type, names: &[T]) -> bool {
    names.iter().any(|name| type_is(ty, name.as_ref()))
}

#[allow(dead_code)]
pub fn is_vec(ty: &Type) -> bool {
    is_contained_by(ty, "Vec")
}

#[allow(dead_code)]
pub fn is_option(ty: &Type) -> bool {
    is_contained_by(ty, "Option")
}

#[allow(dead_code)]
pub fn is_number(ty: &Type) -> bool {
    type_is_in(
        ty,
        &[
            "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
        ],
    )
}

#[allow(dead_code)]
pub fn is_bool(ty: &Type) -> bool {
    type_is(ty, "bool")
}

#[allow(dead_code)]
pub fn is_string(ty: &Type) -> bool {
    type_is(ty, "String")
}

#[allow(dead_code)]
pub fn is_contained_by<T: AsRef<str>>(ty: &Type, container_type: T) -> bool {
    let container_type = container_type.as_ref();
    extract_last_path_segment(ty)
        .map(|path_seg| path_seg.ident == container_type)
        .unwrap_or(false)
}

#[allow(dead_code)]
pub fn unwrap_vec(ty: &Type) -> &Type {
    unwrap_generic_type(ty, "Vec")
}

#[allow(dead_code)]
pub fn unwrap_generic_type<T: AsRef<str>>(ty: &Type, container_type: T) -> &Type {
    let container_type = container_type.as_ref();
    extract_last_path_segment(ty)
        .and_then(|path_seg| {
            if path_seg.ident != container_type {
                return None;
            }
            match path_seg.arguments {
                PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    colon2_token: _,
                    lt_token: _,
                    ref args,
                    gt_token: _,
                }) => args.first().and_then(|a| match a {
                    &GenericArgument::Type(ref inner_ty) => Some(inner_ty),
                    _ => None,
                }),
                _ => None,
            }
        })
        .unwrap_or(ty)
}

pub fn extract_last_path_segment(ty: &Type) -> Option<&PathSegment> {
    match ty {
        &Type::Path(TypePath {
            qself: _,
            path:
                Path {
                    segments: ref seg,
                    leading_colon: _,
                },
        }) => seg.last(),
        _ => None,
    }
}

pub fn extract_fields(data: &Data) -> &FieldsNamed {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => fields,
            _ => panic!("all fields must be named."),
        },
        _ => panic!("struct expected, but got other item."),
    }
}
