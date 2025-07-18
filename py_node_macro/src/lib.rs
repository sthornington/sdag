use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, token::Comma, ExprPath, Ident, ItemStruct, Type};

/// Attribute macro to generate PyNode boilerplate: TYPE, FIELDS, #[new], etc.
#[proc_macro_attribute]
pub fn py_node(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as PyNodeArgs);
    let mut strukt = parse_macro_input!(item as ItemStruct);
    let struct_name = &strukt.ident;
    // collect field idents and types from struct
    let mut fields = Vec::new();
    for f in &strukt.fields {
        if let Some(ident) = &f.ident {
            if ident == "id" {
                continue;
            }
            let ty = &f.ty;
            fields.push((ident.clone(), ty.clone()));
        }
    }
    let ty_const = &args.type_const;
    let field_names: Vec<String> = args.fields.iter().map(|id| id.to_string()).collect();
    let n = field_names.len();
    let strs: Vec<proc_macro2::TokenStream> = field_names.iter().map(|s| quote! { #s }).collect();
    let field_idents: Vec<Ident> = args.fields.iter().cloned().collect();
    let field_tys: Vec<Type> = fields
        .iter()
        .filter_map(|(id, ty)| {
            if args.fields.iter().any(|f| f == id) {
                Some(ty.clone())
            } else {
                None
            }
        })
        .collect();
    // build signature args tokens
    let sig_args: Vec<proc_macro2::TokenStream> = std::iter::once(quote! { id })
        .chain(field_idents.iter().map(|id| quote! { #id }))
        .collect();
    // build constructor mapping
    let ctor_vals: Vec<proc_macro2::TokenStream> = std::iter::once(quote! {}).chain(field_idents.iter().map(|id| quote! { #id })).collect();
    // generate impl
    let expanded = quote! {
        #strukt
        #[pymethods]
        impl #struct_name {
            #[classattr]
            const TYPE: &'static str = #ty_const;
            #[classattr]
            const FIELDS: [&'static str; #n] = [#(#strs),*];
            #[new]
            #[pyo3(signature = (id, #(#field_idents),*))]
            fn new(id: String, #(#field_idents: #field_tys),*) -> Self {
                #struct_name { id, #(#field_idents),* }
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
