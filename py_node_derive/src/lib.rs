use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Data, DeriveInput, Fields, ItemStruct, Meta, NestedMeta};

/// Attribute macro for Python-node wrappers that auto-generates classattrs:
///   TYPE (from the engine node), FIELDS (all field names), SEQ_FIELDS (Vec-typed fields).
///
/// Usage: #[py_node(engine = EngineNodeImpl)] on the #[pyclass] struct definition.
/// Derive macro to generate a trait that captures field names and sequence fields.
///
/// #[derive(PyNode)] on struct Foo produces:
///   trait _FooPyNodeMeta { FIELDS, SEQ_FIELDS }
///   impl _FooPyNodeMeta for Foo { /* names */ }
#[proc_macro_derive(PyNode)]
pub fn derive_py_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let mut fields = Vec::new();
    let mut seq_fields = Vec::new();
    if let Data::Struct(s) = &input.data {
        if let Fields::Named(named) = &s.fields {
            for f in &named.named {
                if let Some(ident) = &f.ident {
                    let field_name = ident.to_string();
                    fields.push(field_name.clone());
                    if let syn::Type::Path(tp) = &f.ty {
                        if tp.path.segments.last().unwrap().ident == "Vec" {
                            seq_fields.push(field_name.clone());
                        }
                    }
                }
            }
        }
    }
    let f_count = fields.len();
    let sf_count = seq_fields.len();
    let field_strs = fields.iter().map(|s| quote! {#s});
    let seq_strs = seq_fields.iter().map(|s| quote! {#s});
    let trait_ident = syn::Ident::new(&format!("_{}PyNodeMeta", name), Span::call_site());
    let expanded = quote! {
        trait #trait_ident {
            const FIELDS: &'static [&'static str];
            const SEQ_FIELDS: &'static [&'static str];
        }
        impl #trait_ident for #name {
            const FIELDS: &'static [&'static str] = &[#(#field_strs),*];
            const SEQ_FIELDS: &'static [&'static str] = &[#(#seq_strs),*];
        }
    };
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn py_node(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Expect exactly one path argument, e.g. #[py_node(EngineNodeImpl)]
    let args = parse_macro_input!(attr as AttributeArgs);
    let engine_path = if args.len() == 1 {
        match &args[0] {
            NestedMeta::Meta(Meta::Path(p)) => p.clone(),
            _ => {
                return syn::Error::new_spanned(
                    &args[0], "expected a single path argument for #[py_node]")
                    .to_compile_error()
                    .into()
            }
        }
    } else {
        return syn::Error::new(proc_macro2::Span::call_site(),
            "expected one argument for #[py_node]")
            .to_compile_error()
            .into();
    };
    // Parse the struct to extract field names and Vec markers
    let input = parse_macro_input!(item as ItemStruct);
    let struct_ident = &input.ident;
    let mut fields = Vec::new();
    let mut seq_fields = Vec::new();
    if let syn::Fields::Named(ref named) = input.fields {
        for f in &named.named {
            if let Some(ident) = &f.ident {
                let name = ident.to_string();
                fields.push(name.clone());
                if let syn::Type::Path(tp) = &f.ty {
                    if tp.path.segments.last().unwrap().ident == "Vec" {
                        seq_fields.push(name.clone());
                    }
                }
            }
        }
    }
    // Build arrays
    let f_count = fields.len();
    let sf_count = seq_fields.len();
    let field_strs = fields.iter().map(|s| quote! { #s });
    let seq_strs = seq_fields.iter().map(|s| quote! { #s });
    // Generate the impl block with classattrs
    let gen = quote! {
        #input
        #[pymethods]
        impl #struct_ident {
            #[classattr]
            const TYPE: &'static str = #engine_path::TYPE;
            #[classattr]
            const FIELDS: [&'static str; #f_count] = [#(#field_strs),*];
            #[classattr]
            const SEQ_FIELDS: [&'static str; #sf_count] = [#(#seq_strs),*];
        }
    };
    gen.into()
}
