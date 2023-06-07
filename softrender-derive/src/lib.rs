extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::{Data, DeriveInput};

#[proc_macro_derive(Barycentric)]
pub fn barycentric_impl(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    let Data::Struct(data) = ast.data else { unimplemented!() };

    let struct_name = ast.ident;
    // First we construct the body of the impl, line-by-line, to simply recursively call
    // interpolated on every field it contains, using that same field in the other two
    // scalars provided (second and third)
    let fields = data.fields.iter().map(|field| {
        let field_name = &field.ident;
        quote::quote!(
         #field_name: self.#field_name.interpolated(lambda, &second.#field_name, &third.#field_name))
    });

    // Once all the individual recursive calls are constructed, we construct the impl itself,
    // returning a new instance of struct_name with the result of the recursive calls to interpolated
    // as its fields
    let expanded = quote::quote!(
      impl Barycentric for #struct_name {
         fn interpolated(&self, lambda: glam::Vec3, second: &Self, third: &Self) -> Self {
            #struct_name {
               #(#fields,)*
            }
         }
      }
    );

    TokenStream::from(expanded)
}
