//! Procedural macros for SchemaSync
//!
//! This crate provides the #[schema_sync] attribute macro and SchemaSync derive macro
//! for model registration with the schema_sync library.

use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields};
use std::sync::Mutex;

/// Registry for models that are decorated with the #[schema_sync] attribute
static MODEL_REGISTRY: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Attribute macro for marking structs to be included in schema generation
#[proc_macro_attribute]
pub fn schema_sync(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(item as DeriveInput);
    let name = input.ident.to_string();
    
    // Register this model
    MODEL_REGISTRY.lock().unwrap().push(name.clone());
    
    // Parse attribute arguments
    let attr_args = parse_attribute_args(proc_macro2::TokenStream::from(attr));
    
    // Generate the modified struct with additional attributes
    let expanded = expand_struct(input, attr_args);
    
    proc_macro::TokenStream::from(expanded)
}

/// Parse attribute arguments like table name, indexes, etc.
fn parse_attribute_args(attr: TokenStream2) -> Vec<(String, String)> {
    // For the basic implementation, just returning empty vec
    // In a real implementation, this would parse arguments like:
    // #[schema_sync(table = "users", index = ["email", "username"])]
    Vec::new()
}

/// Expand the struct definition with required traits and methods
fn expand_struct(input: DeriveInput, attr_args: Vec<(String, String)>) -> TokenStream2 {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    
    // Extract field information for schema generation
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("SchemaSync only supports structs with named fields"),
        },
        _ => panic!("SchemaSync only supports structs"),
    };
    
    // Generate implementation of SchemaSync trait
    let expanded = quote! {
        // Original struct
        #input
        
        #[automatically_derived]
        impl #impl_generics schema_sync::models::SchemaSyncModel for #name #ty_generics #where_clause {
            fn get_table_name() -> String {
                // In a real implementation, this would use the table name from attribute args
                // or apply naming conventions from config
                stringify!(#name).to_string()
            }
            
            fn get_field_definitions() -> Vec<schema_sync::schema::types::FieldDefinition> {
                // In a real implementation, this would extract field types and attributes
                vec![]
            }
            
            fn register_with_schema_sync() {
                // Registration logic
            }
        }
    };
    
    expanded
}

/// Derive macro for SchemaSync
#[proc_macro_derive(SchemaSync, attributes(schema_sync_field))]
pub fn derive_schema_sync(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let expanded = quote! {
        impl schema_sync::models::SchemaSyncModel for #name {
            // Implementation details
            fn get_table_name() -> String {
                stringify!(#name).to_string()
            }
            
            fn get_field_definitions() -> Vec<schema_sync::schema::types::FieldDefinition> {
                vec![]
            }
            
            fn register_with_schema_sync() {
                // Registration logic
            }
        }
    };
    
    TokenStream::from(expanded)
}