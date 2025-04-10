extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemStruct};

mod discriminator;
mod hasher;

#[proc_macro_derive(LightDiscriminator)]
pub fn light_discriminator(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    discriminator::discriminator(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Makes the annotated struct hashable by implementing the following traits:
///
/// - [`AsByteVec`](light_hasher::bytes::AsByteVec), which makes the struct
///   convertable to a 2D byte vector.
/// - [`DataHasher`](light_hasher::DataHasher), which makes the struct hashable
///   with the `hash()` method, based on the byte inputs from `AsByteVec`
///   implementation.
///
/// This macro assumes that all the fields of the struct implement the
/// `AsByteVec` trait. The trait is implemented by default for the most of
/// standard Rust types (primitives, `String`, arrays and options carrying the
/// former). If there is a field of a type not implementing the trait, there
/// are two options:
///
/// 1. The most recommended one - annotating that type with the `light_hasher`
///    macro as well.
/// 2. Manually implementing the `AsByteVec` trait.
///
/// # Attributes
///
/// - `skip` - skips the given field, it doesn't get included neither in
///   `AsByteVec` nor `DataHasher` implementation.
/// - `hash` - makes sure that the byte value does not exceed the BN254
///   prime field modulus, by hashing it (with Keccak) and truncating it to 31
///   bytes. It's generally a good idea to use it on any field which is
///   expected to output more than 31 bytes.
///
/// # Examples
///
/// Compressed account with only primitive types as fields:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64,
///     b: Option<u64>,
/// }
/// ```
///
/// Compressed account with fields which might exceed the BN254 prime field:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64
///     b: Option<u64>,
///     #[hash]
///     c: [u8; 32],
///     #[hash]
///     d: String,
/// }
/// ```
///
/// Compressed account with fields we want to skip:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64
///     b: Option<u64>,
///     #[skip]
///     c: [u8; 32],
/// }
/// ```
///
/// Compressed account with a nested struct:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64
///     b: Option<u64>,
///     c: MyStruct,
/// }
///
/// #[derive(LightHasher)]
/// pub struct MyStruct {
///     a: i32
///     b: u32,
/// }
/// ```
///
/// Compressed account with a type with a custom `AsByteVec` implementation:
///
/// ```ignore
/// #[derive(LightHasher)]
/// pub struct MyCompressedAccount {
///     a: i64
///     b: Option<u64>,
///     c: RData,
/// }
///
/// pub enum RData {
///     A(Ipv4Addr),
///     AAAA(Ipv6Addr),
///     CName(String),
/// }
///
/// impl AsByteVec for RData {
///     fn as_byte_vec(&self) -> Vec<Vec<u8>> {
///         match self {
///             Self::A(ipv4_addr) => vec![ipv4_addr.octets().to_vec()],
///             Self::AAAA(ipv6_addr) => vec![ipv6_addr.octets().to_vec()],
///             Self::CName(cname) => cname.as_byte_vec(),
///         }
///     }
/// }
/// ```
#[proc_macro_derive(LightHasher, attributes(skip, hash, flatten))]
pub fn light_hasher(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    hasher::hasher(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
