<!-- cargo-rdme start -->

# Light Token SDK

The base library to use Light Token Accounts, Light Mints, and compressed token accounts.

## Light Token Accounts
- are on Solana devnet.
- are Solana accounts.
- are functionally equivalent to SPL token accounts.
- can hold tokens of Light, SPL and Token 2022 mints.
- cost 17,288 lamports to create with 24 hours rent.
- are rentfree:
    - rent exemption is sponsored by the token program.
    - rent is 388 lamports per rent epoch (1.5 hours).
    - once the account's lamports balance is insufficient, it is auto-compressed to a compressed token account.
    - the accounts state is cryptographically preserved on the Solana ledger.
    - compressed tokens can be decompressed to a Light Token account.
    - configurable lamports per write (eg transfer) keep the Light Token account perpetually funded when used. So you don't have to worry about funding rent.
    - users load a compressed account into a light account in-flight when using the account again.

## Light Mints
- are on Solana devnet.
- are Compressed accounts.
- cost 15,000 lamports to create.
- support `TokenMetadata`.
- have the same rent-config as light token accounts

## Compressed Token Accounts
- are on Solana mainnet.
- are compressed accounts.
- can hold Light Mint and SPL Mint tokens.
- cost 5,000 lamports to create.
- are well suited for airdrops and reward distribution.

For full program examples, see the [Light Token Examples](https://github.com/Lightprotocol/examples-light-token).

| Operation | Docs guide | GitHub example |
|-----------|-----------|----------------|
| `CreateAssociatedAccountCpi` | [create-ata](https://zkcompression.com/light-token/cookbook/create-ata) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/create-ata) |
| `CreateTokenAccountCpi` | [create-token-account](https://zkcompression.com/light-token/cookbook/create-token-account) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/create-token-account) |
| `CreateMintCpi` | [create-mint](https://zkcompression.com/light-token/cookbook/create-mint) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/create-mint) |
| `MintToCpi` | [mint-to](https://zkcompression.com/light-token/cookbook/mint-to) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/mint-to) |
| `MintToCheckedCpi` | [mint-to](https://zkcompression.com/light-token/cookbook/mint-to) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/mint-to-checked) |
| `BurnCpi` | [burn](https://zkcompression.com/light-token/cookbook/burn) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/burn) |
| `TransferCheckedCpi` | [transfer-checked](https://zkcompression.com/light-token/cookbook/transfer-checked) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/transfer-checked) |
| `TransferInterfaceCpi` | [transfer-interface](https://zkcompression.com/light-token/cookbook/transfer-interface) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/transfer-interface) |
| `ApproveCpi` | [approve-revoke](https://zkcompression.com/light-token/cookbook/approve-revoke) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/approve) |
| `RevokeCpi` | [approve-revoke](https://zkcompression.com/light-token/cookbook/approve-revoke) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/revoke) |
| `FreezeCpi` | [freeze-thaw](https://zkcompression.com/light-token/cookbook/freeze-thaw) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/freeze) |
| `ThawCpi` | [freeze-thaw](https://zkcompression.com/light-token/cookbook/freeze-thaw) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/thaw) |
| `CloseAccountCpi` | [close-token-account](https://zkcompression.com/light-token/cookbook/close-token-account) | [example](https://github.com/Lightprotocol/examples-light-token/tree/main/program-examples/anchor/basic-instructions/close-token-account) |

## Features

1. anchor - Derives AnchorSerialize, AnchorDeserialize instead of BorshSerialize, BorshDeserialize.
2. compressible - utility functions for compressible sdk macros.

## Common Operations

| Operation | Instruction Builder | CPI Builder |
|-----------|----------------|-------------|
| Create Associated Token Account | [`CreateAssociatedTokenAccount`](instruction::CreateAssociatedTokenAccount) | [`CreateAssociatedAccountCpi`](instruction::CreateAssociatedAccountCpi) |
| Create Token Account | [`CreateTokenAccount`](instruction::CreateTokenAccount) | [`CreateTokenAccountCpi`](instruction::CreateTokenAccountCpi) |
| Transfer | [`Transfer`](instruction::Transfer) | [`TransferCpi`](instruction::TransferCpi) |
| Transfer Interface (auto-detect) | [`TransferInterface`](instruction::TransferInterface) | [`TransferInterfaceCpi`](instruction::TransferInterfaceCpi) |
| Close Token account | [`CloseAccount`](instruction::CloseAccount) | [`CloseAccountCpi`](instruction::CloseAccountCpi) |
| Create Mint | [`CreateMint`](instruction::CreateMint) | [`CreateMintCpi`](instruction::CreateMintCpi) |
| MintTo | [`MintTo`](instruction::MintTo) | [`MintToCpi`](instruction::MintToCpi) |


# Disclaimer
This library is not audited and in a beta state. Use at your own risk and expect breaking changes.

<!-- cargo-rdme end -->
