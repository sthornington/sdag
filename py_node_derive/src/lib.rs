use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Data, DeriveInput, Fields, Meta, NestedMeta};

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
    // Single argument is the engine node path: #[py_node(FooNodeImpl)]
    let args = parse_macro_input!(attr as AttributeArgs);
    let engine = if args.len() == 1 {
        match &args[0] {
            NestedMeta::Meta(Meta::Path(p)) => p.clone(),
            other => return syn::Error::new_spanned(other, "expected single type path").to_compile_error().into(),
        }
    } else {
        return syn::Error::new(proc_macro2::Span::call_site(), "expected one argument for #[py_node]")
            .to_compile_error().into();
    };

    // Parse existing impl block and inject the three classattrs
    let mut imp = parse_macro_input!(item as syn::ItemImpl);
    // Determine the impl target type name (e.g. Foo)
    let struct_ident = if let syn::Type::Path(tp) = &*imp.self_ty {
        tp.path.segments.last().unwrap().ident.clone()
    } else {
        return syn::Error::new_spanned(&imp.self_ty, "unsupported type for #[py_node]")
            .to_compile_error()
            .into();
    };
    // Build the three classattr const items
    let meta_trait = syn::Ident::new(&format!("_{}PyNodeMeta", struct_ident), Span::call_site());
    let type_item: syn::ImplItemConst = syn::parse2(quote! {
        #[classattr]
        const TYPE: &'static str = #engine::TYPE;
    }).unwrap();
    let fields_item: syn::ImplItemConst = syn::parse2(quote! {
        #[classattr]
        const FIELDS: &'static [&'static str] = <#struct_ident as #meta_trait>::FIELDS;
    }).unwrap();
    let seq_item: syn::ImplItemConst = syn::parse2(quote! {
        #[classattr]
        const SEQ_FIELDS: &'static [&'static str] = <#struct_ident as #meta_trait>::SEQ_FIELDS;
    }).unwrap();
    // Prepend consts to the impl
    imp.items.splice(0..0, vec![
        syn::ImplItem::Const(type_item),
        syn::ImplItem::Const(fields_item),
        syn::ImplItem::Const(seq_item),
    ]);
    TokenStream::from(quote!(#imp))
}
