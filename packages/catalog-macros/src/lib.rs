use proc_macro::TokenStream;
use quote::quote;
use syn::{Item, parse_macro_input};

/// Procedural macro to automatically register nodes in the catalog.
///
/// Usage:
/// ```
/// #[register_node]
/// #[derive(Default)]
/// pub struct MyNode {}
/// ```
///
/// This will automatically register the node when the catalog is initialized.
#[proc_macro_attribute]
pub fn register_node(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as Item);

    let (struct_item, name) = match &input {
        Item::Struct(item_struct) => (input.clone(), item_struct.ident.clone()),
        _ => panic!("register_node can only be used on structs"),
    };

    let expanded = quote! {
        #struct_item

        ::inventory::submit! {
            #[allow(clippy::redundant_closure)]
            crate::NodeConstructor::new(|| {
                ::std::sync::Arc::new(#name::default()) as ::std::sync::Arc<dyn ::flow_like::flow::node::NodeLogic>
            })
        }
    };

    TokenStream::from(expanded)
}
