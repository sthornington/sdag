use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, token::Comma, ExprPath, Ident, ItemStruct, Type};

/// Attribute macro to generate PyNode boilerplate: TYPE, FIELDS, #[new], etc.
#[proc_macro_attribute]
pub fn py_node(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the wrapper struct and its declared fields
    let args = parse_macro_input!(attr as PyNodeArgs);
    let wrapper = parse_macro_input!(item as ItemStruct);
    let wrapper_name = &wrapper.ident;
    let ty_const = &args.type_const;

    // Gather wrapper payload fields (skip `id`) in declared order
    let field_idents: Vec<Ident> = args.fields.iter().cloned().collect();
    let mut wrapper_fields = Vec::new();
    for f in &wrapper.fields {
        if let Some(id) = &f.ident {
            if id == "id" { continue; }
            wrapper_fields.push((id.clone(), f.ty.clone()));
        }
    }
    // Match wrapper types to declared fields
    let wrapper_tys: Vec<Type> = field_idents.iter().map(|id| {
        wrapper_fields.iter()
            .find(|(fid, _)| fid == id).map(|(_, ty)| ty.clone())
            .expect("field in attr must exist in struct")
    }).collect();

    // Map wrapper types to spec types: PyObject→String, Vec<PyObject>→Vec<String>
    fn map_spec_ty(ty: &Type) -> Type {
        if let Type::Path(p) = ty {
            if p.path.segments.len() == 1 && p.path.segments[0].ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(gen) = &p.path.segments[0].arguments {
                    if let Some(syn::GenericArgument::Type(Type::Path(inner))) = gen.args.first() {
                        if inner.path.is_ident("PyObject") {
                            return syn::parse_str("Vec<String>").unwrap();
                        }
                    }
                }
            }
            if p.path.is_ident("PyObject") {
                return syn::parse_str("String").unwrap();
            }
        }
        ty.clone()
    }
    let spec_tys: Vec<Type> = wrapper_tys.iter().map(map_spec_ty).collect();
    // Flags for generating the correct builder logic
    let is_many: Vec<bool> = wrapper_tys.iter().map(|ty| {
        if let Type::Path(p) = ty {
            p.path.segments.len() == 1 && p.path.segments[0].ident == "Vec"
        } else {
            false
        }
    }).collect();
    let is_one: Vec<bool> = wrapper_tys.iter().map(|ty| {
        if let Type::Path(p) = ty {
            p.path.is_ident("PyObject")
        } else {
            false
        }
    }).collect();


    // Build spec struct name (e.g. Add → AddSpec) and engine type ident (e.g. AddNode)
    let spec_name = Ident::new(&format!("{}Spec", wrapper_name), wrapper_name.span());
    let engine_ty = args.type_const.path.segments.first().expect("type const path").ident.clone();

    // Prepare serde-renamed field names and FIELDS const
    let field_strs: Vec<String> = field_idents.iter().map(|id| id.to_string()).collect();
    let nfields = field_strs.len();

    // Prepare builder snippets per field
    let builder_fields: Vec<proc_macro2::TokenStream> = field_idents
        .iter()
        .zip(is_many.iter())
        .zip(is_one.iter())
        .map(|((ident, &many), &one)| {
            if many {
                quote! {
                    let #ident = {
                        let mut out = Vec::new();
                        for id in spec.#ident.clone() {
                            out.push(crate::engine::build_node(&serde_yaml::Value::String(id))?);
                        }
                        out
                    };
                }
            } else if one {
                quote! {
                    let #ident = crate::engine::build_node(&serde_yaml::Value::String(spec.#ident.clone()))?;
                }
            } else {
                quote! {
                    let #ident = spec.#ident.clone();
                }
            }
        })
        .collect();

    // Generate spec struct, builder impl, inventory registration, and Python wrapper
    let expanded = quote! {
        /// Internal spec for YAML serialization and engine building
        #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
        #[serde(crate = "serde", deny_unknown_fields)]
        pub struct #spec_name {
            pub id: String,
            #(
                #[serde(rename = #field_strs)]
                pub #field_idents: #spec_tys,
            )*
        }

        impl crate::engine::NodeDef for #spec_name {
            const TYPE: &'static str = #ty_const;
            fn from_yaml(v: &serde_yaml::Value) -> Result<Box<dyn crate::engine::Node + Send + Sync>, String> {
                let spec: Self = serde_yaml::from_value(v.clone()).map_err(|e| e.to_string())?;
                // build engine node from spec
                #(#builder_fields)*
                let node = crate::engine::#engine_ty { #(#field_idents),* };
                Ok(Box::new(node))
            }
        }
        inventory::submit! {
            crate::engine::Builder { tag: #spec_name::TYPE, build: #spec_name::from_yaml }
        }

        // Python wrapper struct and methods
        #wrapper

        #[pymethods]
        impl #wrapper_name {
            #[classattr]
            pub const TYPE: &'static str = #ty_const;
            #[classattr]
            pub const FIELDS: [&'static str; #nfields] = [#(#field_strs),*];
            #[new]
            #[pyo3(signature = (id, #(#field_idents),*))]
            pub fn new(id: String, #(#field_idents: #wrapper_tys),*) -> Self {
                #wrapper_name { id, #(#field_idents),* }
            }
        }
    };
    TokenStream::from(expanded)
}

/// Parse attribute arguments: first is type const (ExprPath), then comma-separated idents
struct PyNodeArgs {
    type_const: ExprPath,
    _comma: Comma,
    fields: Punctuated<Ident, Comma>,
}
impl Parse for PyNodeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let type_const: ExprPath = input.parse()?;
        let _comma: Comma = input.parse()?;
        let fields = Punctuated::parse_separated_nonempty(input)?;
        Ok(PyNodeArgs { type_const, _comma, fields })
    }
}
