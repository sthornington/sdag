use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(SdagNode, attributes(sdag))]
pub fn derive_sdag_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    // Get the struct name (e.g., InputNode)
    let struct_name = &input.ident;
    
    // Extract the tag from #[sdag(tag = "...")] attribute
    let tag = extract_tag(&input.attrs);
    
    // Derive the Python class name by removing "Node" suffix
    let py_class_name = struct_name.to_string().trim_end_matches("Node").to_string();
    let py_class_ident = syn::Ident::new(&py_class_name, struct_name.span());
    
    // Extract fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("SdagNode only supports structs with named fields"),
        },
        _ => panic!("SdagNode can only be used on structs"),
    };
    
    // Generate field definitions for Python class
    let py_fields = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        let py_type = rust_type_to_py_type(field_type);
        
        quote! {
            #[pyo3(get)]
            pub #field_name: #py_type,
        }
    });
    
    // Generate ArenaEval implementation
    let arena_eval_impl = quote! {
        impl crate::ArenaEval for #struct_name {
            fn eval_arena(&self, values: &[f64], inputs: &std::collections::HashMap<String, f64>) -> f64 {
                <Self as crate::EvalNode>::eval(self, values, inputs)
            }
        }
    };
    
    // Generate Python wrapper class
    let py_class = quote! {
        #[::pyo3::pyclass]
        pub struct #py_class_ident {
            #[pyo3(get)]
            pub id: String,
            #(#py_fields)*
        }
    };
    
    // Generate from_arena method
    let from_arena_fields = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        let field_name_str = field_name.as_ref().unwrap().to_string();
        
        let extraction = match field_type_category(field_type) {
            FieldCategory::String => quote! {
                let #field_name = match node.fields.get(#field_name_str) {
                    Some(crate::engine::FieldValue::Str(s)) => s.clone(),
                    _ => return Err(format!("Missing field '{}'", #field_name_str)),
                };
            },
            FieldCategory::Float => quote! {
                let #field_name = match node.fields.get(#field_name_str) {
                    Some(crate::engine::FieldValue::Float(f)) => *f,
                    _ => return Err(format!("Missing field '{}'", #field_name_str)),
                };
            },
            FieldCategory::NodeId => quote! {
                let #field_name = match node.fields.get(#field_name_str) {
                    Some(crate::engine::FieldValue::One(id)) => *id,
                    _ => return Err(format!("Missing field '{}'", #field_name_str)),
                };
            },
            FieldCategory::VecNodeId => quote! {
                let #field_name = match node.fields.get(#field_name_str) {
                    Some(crate::engine::FieldValue::Many(ids)) => ids.clone(),
                    _ => return Err(format!("Missing field '{}'", #field_name_str)),
                };
            },
        };
        
        extraction
    });
    
    let field_names = fields.iter().map(|f| &f.ident);
    
    let from_arena = quote! {
        impl #struct_name {
            pub fn from_arena(node: &crate::engine::ArenaNode) -> Result<Self, String> {
                #(#from_arena_fields)*
                
                Ok(Self {
                    #(#field_names,)*
                })
            }
        }
    };
    
    // Combine everything
    let expanded = quote! {
        #arena_eval_impl
        #py_class
        #from_arena
    };
    
    TokenStream::from(expanded)
}

fn extract_tag(attrs: &[syn::Attribute]) -> String {
    for attr in attrs {
        if attr.path().is_ident("sdag") {
            let result: Result<String, syn::Error> = attr.parse_args_with(|input: syn::parse::ParseStream| {
                let ident: syn::Ident = input.parse()?;
                if ident != "tag" {
                    return Err(syn::Error::new(ident.span(), "expected 'tag'"));
                }
                input.parse::<syn::Token![=]>()?;
                let lit: syn::LitStr = input.parse()?;
                Ok(lit.value())
            });
            
            if let Ok(tag) = result {
                return tag;
            }
        }
    }
    panic!("SdagNode requires #[sdag(tag = \"...\")] attribute");
}

enum FieldCategory {
    String,
    Float,
    NodeId,
    VecNodeId,
}

fn field_type_category(ty: &syn::Type) -> FieldCategory {
    let type_str = quote!(#ty).to_string();
    if type_str == "String" {
        FieldCategory::String
    } else if type_str == "f64" {
        FieldCategory::Float
    } else if type_str == "NodeId" || type_str == "crate :: engine :: NodeId" {
        FieldCategory::NodeId
    } else if type_str.contains("Vec < NodeId >") || type_str.contains("Vec < crate :: engine :: NodeId >") {
        FieldCategory::VecNodeId
    } else {
        panic!("Unsupported field type: {}", type_str);
    }
}

fn rust_type_to_py_type(ty: &syn::Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string();
    if type_str == "String" || type_str == "f64" {
        quote!(#ty)
    } else if type_str == "NodeId" || type_str.contains("NodeId") {
        if type_str.starts_with("Vec") {
            quote!(Vec<::pyo3::PyObject>)
        } else {
            quote!(::pyo3::PyObject)
        }
    } else {
        quote!(#ty)
    }
}