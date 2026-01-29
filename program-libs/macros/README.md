<!-- cargo-rdme start -->

# light-macros

Proc macros for Light Protocol on-chain programs.

| Macro | Description |
|-------|-------------|
| [`pubkey!`](macro@pubkey) | Convert base58 public key to `Pubkey` at compile time |
| [`pubkey_array!`](macro@pubkey_array) | Convert base58 public key to `[u8; 32]` at compile time |
| [`derive_light_cpi_signer!`](macro@derive_light_cpi_signer) | Derive CPI signer PDA, program ID, and bump seed |
| [`derive_light_cpi_signer_pda!`](macro@derive_light_cpi_signer_pda) | Derive CPI signer PDA address and bump seed |
| [`#[heap_neutral]`](macro@heap_neutral) | Assert a function frees all heap it allocates |
| [`#[derive(Noop)]`](derive@Noop) | No-op derive placeholder |

<!-- cargo-rdme end -->
