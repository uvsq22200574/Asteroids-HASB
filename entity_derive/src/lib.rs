extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Entity)]
pub fn entity_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident; // Struct name

    // Generate the trait implementation
    let expanded = quote! {
        impl ast_lib::CosmicEntity for #name {
            fn get_id(&self) -> u64 { self.id }
            fn clone_with_new_uid(&self) -> Self {
                let mut cloned = self.clone();
                cloned.id = ast_lib::generate_uid();
                cloned
            }
            fn get_position(&self) -> ::macroquad::prelude::Vec2 { self.position }
            fn get_speed(&self) -> f32 { self.speed }
            fn get_size(&self) -> f32 { self.size }
            fn get_rotation(&self) -> f32 { self.rotation }
            fn add_rotation(&mut self, amount: f32) {
                self.rotation = (self.rotation - amount) % (std::f32::consts::PI * 2.0);
            }
        }
    };

    // Convert the generated code into a TokenStream
    TokenStream::from(expanded)
}
