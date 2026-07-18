//! Proc-macro crate for Pigeon.
//!
//! The `#[derive(Packet)]` helper arrives in M2.4 — for now we expose
//! a noop so the crate compiles and downstream code continues to type-check.

use proc_macro::TokenStream;

#[proc_macro_derive(Packet)]
pub fn derive_packet(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}
