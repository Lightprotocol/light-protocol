<!-- cargo-rdme start -->

## light-macros

Proc macros for Light Protocol on-chain programs.

| Macro | Description |
|-------|-------------|
| [`pubkey!`](pubkey) | Convert base58 public key to `Pubkey` at compile time |
| [`pubkey_array!`](pubkey_array) | Convert base58 public key to `[u8; 32]` at compile time |
| [`derive_light_cpi_signer!`](derive_light_cpi_signer) | Derive CPI signer PDA, program ID, and bump seed |
| [`derive_light_cpi_signer_pda!`](derive_light_cpi_signer_pda) | Derive CPI signer PDA address and bump seed |
| [`#[heap_neutral]`](heap_neutral) | Assert a function frees all heap it allocates |
| [`#[derive(Noop)]`](derive_noop) | No-op derive placeholder |

<!-- cargo-rdme end -->
