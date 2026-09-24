# Instruction Reference

## Program ID
```
SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7
```

## Discriminator Table

| Instruction | Discriminator | Enum Variant | Source |
|-------------|---------------|--------------|--------|
| InitializeCpiContextAccount | `[233, 112, 71, 66, 121, 33, 178, 188]` | `InstructionDiscriminator::InitializeCpiContextAccount` | `src/accounts/init_context_account.rs` |
| Invoke | `[26, 16, 169, 7, 21, 202, 242, 25]` | `InstructionDiscriminator::Invoke` | `src/invoke/` |
| InvokeCpi | `[49, 212, 191, 129, 39, 194, 43, 196]` | `InstructionDiscriminator::InvokeCpi` | `src/invoke_cpi/` |
| InvokeCpiWithReadOnly | `[86, 47, 163, 166, 21, 223, 92, 8]` | `InstructionDiscriminator::InvokeCpiWithReadOnly` | `src/lib.rs` |
| InvokeCpiWithAccountInfo | `[228, 34, 128, 84, 47, 139, 86, 240]` | `InstructionDiscriminator::InvokeCpiWithAccountInfo` | `src/lib.rs` |
| ReInitCpiContextAccount | `[187, 147, 22, 142, 104, 180, 136, 190]` | `InstructionDiscriminator::ReInitCpiContextAccount` | `src/accounts/init_context_account.rs` (feature: "reinit") |

**Source:** `programs/system/src/constants.rs`

## Instruction Categories

### CPI Context Account Management
Instructions for initializing and migrating CPI context accounts.

| Instruction | Description | Documentation |
|-------------|-------------|---------------|
| InitializeCpiContextAccount | Initialize a new CPI context account (version 2) | [INIT_CPI_CONTEXT_ACCOUNT.md](init/INIT_CPI_CONTEXT_ACCOUNT.md) |
| ReInitCpiContextAccount | Migrate CPI context account from version 1 to version 2 | [REINIT_CPI_CONTEXT_ACCOUNT.md](init/REINIT_CPI_CONTEXT_ACCOUNT.md) |

### Direct Invocation
Direct invocation for single-program compressed account operations.

| Instruction | Description | Documentation |
|-------------|-------------|---------------|
| Invoke | Process compressed accounts directly (no CPI) | [INVOKE.md](invoke/INVOKE.md) |

### CPI Invocation
CPI invocation modes for multi-program compressed account operations.

| Instruction | Description | Documentation |
|-------------|-------------|---------------|
| InvokeCpi | Standard CPI invocation with Anchor-style accounts | [INVOKE_CPI.md](invoke_cpi/INVOKE_CPI.md) |
| InvokeCpiWithReadOnly | CPI with read-only compressed account support | [INVOKE_CPI_WITH_READ_ONLY.md](invoke_cpi/INVOKE_CPI_WITH_READ_ONLY.md) |
| InvokeCpiWithAccountInfo | CPI with dynamic account configuration (V2 mode) | [INVOKE_CPI_WITH_ACCOUNT_INFO.md](invoke_cpi/INVOKE_CPI_WITH_ACCOUNT_INFO.md) |

## Input Combinations

| Instruction | Proof Required | CPI Context | Read-Only Accounts | Read-Only Addresses | Data on Inputs |
|-------------|----------------|-------------|-------------------|---------------------|----------------|
| Invoke | When inputs exist | No | No | No | No (error 6001) |
| InvokeCpi | In execute mode | Optional | No | No | Yes (program-owned) |
| InvokeCpiWithReadOnly | In execute mode | Optional | Yes | Execute mode only | Yes |
| InvokeCpiWithAccountInfo | In execute mode | Optional | Yes | Execute mode only | Yes |

### CPI Context Mode Combinations

| first_set_context | set_context | Behavior | Proof Required |
|-------------------|-------------|----------|----------------|
| true | false | Clear + write to context | No |
| false | true | Append to context | No |
| false | false | Execute with proof | Yes |

---

## Instruction Schema Reminder

Every instruction documentation must include:
- **discriminator** - 8-byte value
- **enum** - `InstructionDiscriminator::*` variant
- **path** - Source file path
- **description** - High-level overview, state changes, usage scenarios
- **instruction_data** - Path to structs, field descriptions
- **Accounts** - Ordered list with signer/writable/checks
- **instruction logic and checks** - Step-by-step processing
- **Errors** - Comprehensive list with codes

## Error Codes Reference (SystemProgramError)

| Code | Error | Description |
|------|-------|-------------|
| 6000 | SumCheckFailed | Lamport sum of inputs + compression != outputs + decompression |
| 6001 | SignerCheckFailed | Authority is not a signer for input compressed accounts |
| 6002 | CpiSignerCheckFailed | Invoking program doesn't match expected signer PDA |
| 6003 | ComputeInputSumFailed | Failed to compute input lamports sum |
| 6004 | ComputeOutputSumFailed | Failed to compute output lamports sum |
| 6005 | ComputeRpcSumFailed | Failed to compute RPC sum |
| 6006 | InvalidAddress | Address validation failed |
| 6007 | DeriveAddressError | Failed to derive address from seed |
| 6008 | CompressedSolPdaUndefinedForCompressSol | Sol pool PDA required for compression |
| 6009 | DecompressLamportsUndefinedForCompressSol | Decompression lamports required |
| 6010 | CompressedSolPdaUndefinedForDecompressSol | Sol pool PDA required for decompression |
| 6011 | DeCompressLamportsUndefinedForDecompressSol | Decompression lamports required |
| 6012 | DecompressRecipientUndefinedForDecompressSol | Recipient required for decompression |
| 6013 | WriteAccessCheckFailed | Program doesn't have write access to Merkle tree |
| 6014 | InvokingProgramNotProvided | Invoking program account missing |
| 6015 | InvalidCapacity | CPI context account capacity invalid |
| 6016 | InvalidMerkleTreeOwner | Merkle tree owner mismatch |
| 6017 | ProofIsNone | ZK proof required but not provided |
| 6018 | ProofIsSome | ZK proof provided but not needed |
| 6019 | EmptyInputs | No inputs, outputs, or addresses provided |
| 6020 | CpiContextAccountUndefined | CPI context account required but not provided |
| 6021 | CpiContextEmpty | CPI context account is empty during execute |
| 6022 | CpiContextMissing | CPI context data missing in instruction |
| 6023 | DecompressionRecipientDefined | Unexpected decompression recipient |
| 6024 | SolPoolPdaDefined | Unexpected sol pool PDA |
| 6025 | AppendStateFailed | Failed to append state to Merkle tree |
| 6026 | InstructionNotCallable | Instruction disabled |
| 6027 | CpiContextFeePayerMismatch | Fee payer mismatch in CPI context |
| 6028 | CpiContextAssociatedMerkleTreeMismatch | Merkle tree mismatch in CPI context |
| 6029 | NoInputs | No input accounts provided |
| 6030 | InputMerkleTreeIndicesNotInOrder | Input Merkle tree indices must be ascending |
| 6031 | OutputMerkleTreeIndicesNotInOrder | Output Merkle tree indices must be ascending |
| 6032 | OutputMerkleTreeNotUnique | Output Merkle trees must be unique per batch |
| 6033 | DataFieldUndefined | Required data field is undefined |
| 6034 | ReadOnlyAddressAlreadyExists | Read-only address already in use |
| 6035 | ReadOnlyAccountDoesNotExist | Read-only account not found |
| 6036 | HashChainInputsLenghtInconsistent | Hash chain length mismatch |
| 6037 | InvalidAddressTreeHeight | Address tree height invalid for proof |
| 6038 | InvalidStateTreeHeight | State tree height invalid for proof |
| 6039 | InvalidArgument | Generic invalid argument |
| 6040 | InvalidAccount | Generic invalid account |
| 6041 | AddressMerkleTreeAccountDiscriminatorMismatch | Wrong address Merkle tree discriminator |
| 6042 | StateMerkleTreeAccountDiscriminatorMismatch | Wrong state Merkle tree discriminator |
| 6043 | ProofVerificationFailed | ZK proof verification failed |
| 6044 | InvalidAccountMode | Account mode not recognized |
| 6045 | InvalidInstructionDataDiscriminator | Unknown instruction discriminator |
| 6046 | NewAddressAssignedIndexOutOfBounds | Assigned index exceeds output count |
| 6047 | AddressIsNone | Address expected but not provided |
| 6048 | AddressDoesNotMatch | Address doesn't match expected value |
| 6049 | CpiContextAlreadySet | CPI context already initialized |
| 6050 | InvalidTreeHeight | Tree height invalid |
| 6051 | TooManyOutputAccounts | Exceeds MAX_OUTPUT_ACCOUNTS (30) |
| 6052 | BorrowingDataFailed | Account data borrow failed |
| 6053 | DuplicateAccountInInputsAndReadOnly | Same account in both inputs and read-only |
| 6054 | CpiContextPassedAsSetContext | CPI context account doesn't exist but passed as set_context |
| 6055 | InvalidCpiContextOwner | CPI context account wrong owner |
| 6056 | InvalidCpiContextDiscriminator | CPI context account wrong discriminator |
| 6057 | InvalidAccountIndex | Account index out of bounds |
| 6058 | AccountCompressionCpiDataExceedsLimit | CPI data exceeds 10KB limit |
| 6059 | AddressOwnerIndexOutOfBounds | Address owner index invalid |
| 6060 | AddressAssignedAccountIndexOutOfBounds | Address assigned index invalid |
| 6061 | OutputMerkleTreeIndexOutOfBounds | Output tree index invalid |
| 6062 | PackedAccountIndexOutOfBounds | Packed account index invalid |
| 6063 | Unimplemented | Feature not yet implemented |
| 6064 | CpiContextDeactivated | CPI context is deactivated |
| 6065 | InputMerkleTreeIndexOutOfBounds | Input tree index invalid |
| 6066 | MissingLegacyMerkleContext | Legacy Merkle context required |

**Source:** `programs/system/src/errors.rs`
