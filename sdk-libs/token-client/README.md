<!-- cargo-rdme start -->

# light-token-client

Rust client for light-token. Each action builds, signs,
and sends the transaction.

| Action | Description |
|--------|-------------|
| [`CreateMint`] | Create a light-token mint with metadata |
| [`CreateAta`] | Create an associated light-token account |
| [`MintTo`] | Mint tokens to a light-token account |
| [`Transfer`] | Transfer light-tokens between accounts |
| [`TransferChecked`] | Transfer with decimal validation |
| [`TransferInterface`] | Transfer between light-token, T22, and SPL accounts |
| [`Approve`] | Approve a delegate |
| [`Revoke`] | Revoke a delegate |
| [`Wrap`] | Wrap SPL/T22 to light-token |
| [`Unwrap`] | Unwrap light-token to SPL/T22 |


<!-- cargo-rdme end -->
