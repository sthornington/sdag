use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(SdagNode, attributes(sdag))]
pub fn derive_sdag_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    // Get the struct name
    let struct_name = &input.ident;
    
    // Extract the Python class name from #[sdag(pyclass = "...")] 
    let py_class_name = extract_pyclass(&input.attrs);
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
        let field_type = rust_type_to_py_type(&f.ty);
        quote! {
            #[pyo3(get)]
            pub #field_name: #field_type,
        }
    });
    
    // Generate field conversions for new method
    let field_conversions = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        
        if is_node_ref(field_type) {
            quote! {
                let #field_name = #field_name.getattr(py, "id")?.extract(py)?;
            }
        } else if is_vec_node_ref(field_type) {
            quote! {
                let #field_name = #field_name.into_iter()
                    .map(|n| n.getattr(py, "id")?.extract(py))
                    .collect::<PyResult<Vec<_>>>()?;
            }
        } else {
            quote! {}
        }
    });
    
    let field_names = fields.iter().map(|f| &f.ident);
    let field_names2 = fields.iter().map(|f| &f.ident);
    
    // Generate Python class
    let expanded = quote! {
        #[::pyo3::pyclass]
        #[derive(Clone)]
        pub struct #py_class_ident {
            #[pyo3(get)]
            pub id: usize,
            #(#py_fields)*
        }
        
        #[::pyo3::pymethods]
        impl #py_class_ident {
            #[new]
            fn new(graph: &mut crate::Graph, py: ::pyo3::Python, #(#field_names: ::pyo3::PyObject),*) -> ::pyo3::PyResult<Self> {
                #(#field_conversions)*
                
                // Register with graph
                let id = graph.nodes.len();
                let node = #struct_name { #(#field_names2),* };
                graph.arena.push(Box::new(node));
                
                let py_node = Self {
                    id,
                    #(#field_names2),*
                };
                
                graph.nodes.push(py_node.clone().into_py(py));
                Ok(py_node)
            }
        }
        
        // Register with inventory for module initialization
        inventory::submit! {
            crate::NodeRegistration {
                name: #py_class_name,
                register: |m: &::pyo3::types::PyModule| -> ::pyo3::PyResult<()> {
                    m.add_class::<#py_class_ident>()?;
                    Ok(())
                },
            }
        }
    };
    
    TokenStream::from(expanded)
}

fn extract_pyclass(attrs: &[syn::Attribute]) -> String {
    for attr in attrs {
        if attr.path().is_ident("sdag") {
            let result: Result<String, syn::Error> = attr.parse_args_with(|input: syn::parse::ParseStream| {
                let ident: syn::Ident = input.parse()?;
                if ident != "pyclass" {
                    return Err(syn::Error::new(ident.span(), "expected 'pyclass'"));
                }
                input.parse::<syn::Token![=]>()?;
                let lit: syn::LitStr = input.parse()?;
                Ok(lit.value())
            });
            
            if let Ok(name) = result {
                return name;
            }
        }
    }
    panic!("SdagNode requires #[sdag(pyclass = \"...\")] attribute");
}

fn rust_type_to_py_type(ty: &syn::Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string();
    if type_str == "String" || type_str == "f64" {
        quote!(#ty)
    } else if type_str.contains("NodeId") {
        if type_str.starts_with("Vec") {
            quote!(Vec<::pyo3::PyObject>)
        } else {
            quote!(::pyo3::PyObject)
        }
    } else {
        quote!(#ty)
    }
}

fn is_node_ref(ty: &syn::Type) -> bool {
    let type_str = quote!(#ty).to_string();
    type_str.contains("NodeId") && !type_str.starts_with("Vec")
}

fn is_vec_node_ref(ty: &syn::Type) -> bool {
    let type_str = quote!(#ty).to_string();
    type_str.contains("Vec") && type_str.contains("NodeId")
}