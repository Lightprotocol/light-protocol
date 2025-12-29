# Restricted Token-2022 Extensions

This document describes the behavior of the 5 restricted Token-2022 extensions as implemented in SPL Token-2022. These extensions are classified as "restricted" because they require special handling during compression operations.

## Quick Reference

| Instruction       | TransferFee     | DefaultState        | PermanentDelegate | TransferHook  | Pausable          |
|-------------------|-----------------|---------------------|-------------------|---------------|-------------------|
| InitializeAccount | adds FeeAmount  | applies frozen state| -                 | adds marker   | adds marker       |
| Transfer          | fee deducted   | frozen blocked      | authority check   | CPI invoked  | blocked if paused |
| Approve           | -              | frozen blocked      | owner only        | -            | allowed           |
| Revoke            | -              | frozen blocked      | owner only        | -            | allowed           |
| Burn              | -              | frozen blocked      | authority check   | -            | blocked if paused |
| MintTo            | -              | -                   | -                 | -            | blocked if paused |
| CloseAccount      | withheld check | -                   | -                 | -            | -                 |
| Freeze/Thaw       | -              | -                   | -                 | -            | allowed           |

---

## 1. TransferFeeConfig

### Overview

The TransferFeeConfig extension enables mints to automatically assess fees on token transfers, with fees calculated as a percentage (basis points) of the transfer amount, capped at a configurable maximum. Fees are withheld in the destination account and can be collected by a designated authority.

### Data Structures

#### TransferFeeConfig (Mint Extension)

```rust
pub struct TransferFeeConfig {
    /// Authority that can update the fee configuration
    pub transfer_fee_config_authority: OptionalNonZeroPubkey,
    /// Authority that can withdraw withheld fees
    pub withdraw_withheld_authority: OptionalNonZeroPubkey,
    /// Accumulated fees harvested from accounts, awaiting withdrawal
    pub withheld_amount: PodU64,
    /// Fee schedule used when current_epoch < newer_transfer_fee.epoch
    pub older_transfer_fee: TransferFee,
    /// Fee schedule used when current_epoch >= newer_transfer_fee.epoch
    pub newer_transfer_fee: TransferFee,
}
```

#### TransferFee

```rust
pub struct TransferFee {
    /// Epoch when this fee schedule becomes active
    pub epoch: PodU64,
    /// Maximum fee in token amount (absolute cap)
    pub maximum_fee: PodU64,
    /// Fee rate in basis points (0.01% increments, max 10,000 = 100%)
    pub transfer_fee_basis_points: PodU16,
}
```

#### TransferFeeAmount (Account Extension)

```rust
pub struct TransferFeeAmount {
    /// Fees withheld on this account from incoming transfers
    pub withheld_amount: PodU64,
}
```

### Instruction Behavior

#### Transfer (TransferCheckedWithFee)

**Fee Calculation:**
```
fee = ceil(amount * transfer_fee_basis_points / 10,000)
fee = min(fee, maximum_fee)
```

The ceiling division ensures the protocol never undercharges.

**Token Flow:**
- Source account: debited full `amount`
- Destination account balance: credited `amount - fee`
- Destination account `withheld_amount`: increased by `fee`

The client must provide the expected `fee` parameter, which is validated against the on-chain calculation.

#### CloseAccount

Blocked if `withheld_amount > 0`. Returns `TokenError::AccountHasWithheldTransferFees`. Fees must be harvested or withdrawn before closing.

#### HarvestWithheldTokensToMint

- **Permissionless** - anyone can call this instruction
- Moves `withheld_amount` from specified token accounts to the mint's `withheld_amount`
- Works on frozen accounts

#### WithdrawWithheldTokensFromMint

- Requires signature from `withdraw_withheld_authority`
- Transfers mint's `withheld_amount` to a specified destination token account
- Destination must not be frozen

#### SetTransferFee

- Requires signature from `transfer_fee_config_authority`
- **2-epoch delay**: new fee takes effect at `current_epoch + 2`
- Prevents "rug pulls" where fees could be changed at epoch boundaries

### Validation Rules

| Rule | Error |
|------|-------|
| `transfer_fee_basis_points > 10,000` | `TransferFeeExceedsMaximum` |
| Fee mismatch during transfer | `FeeMismatch` |
| Close account with `withheld_amount > 0` | `AccountHasWithheldTransferFees` |
| Withdraw to frozen account | `AccountFrozen` |
| Missing authority for SetTransferFee/Withdraw | `NoAuthorityExists` |
| Transfer without mint when TransferFeeAmount exists | `MintRequiredForTransfer` |

---

## 2. DefaultAccountState

### Overview

The DefaultAccountState extension allows mint authorities to configure new token accounts to be created in a specific state (Initialized or Frozen) by default. This enables scenarios such as requiring KYC verification before users can interact with their tokens.

### Data Structures

#### DefaultAccountState (Mint Extension)

```rust
pub struct DefaultAccountState {
    /// Default Account::state in which new Accounts should be initialized
    pub state: PodAccountState,  // u8
}
```

#### AccountState Enum

```rust
pub enum AccountState {
    /// Account is not yet initialized (value: 0)
    Uninitialized,
    /// Account is initialized; permitted operations allowed (value: 1)
    Initialized,
    /// Account has been frozen by the mint freeze authority (value: 2)
    Frozen,
}
```

### Instruction Behavior

#### Initialize

- Must be called before `InitializeMint`
- Sets the default state for all new token accounts
- Cannot set state to `Uninitialized`

#### InitializeAccount Interaction

New accounts automatically inherit the mint's default state:
```rust
let starting_state = if let Ok(default_account_state) = mint.get_extension::<DefaultAccountState>() {
    AccountState::try_from(default_account_state.state)?
} else {
    AccountState::Initialized
};
```

#### Operations Blocked When Account is Frozen

| Operation | Source Account | Destination Account |
|-----------|----------------|---------------------|
| Transfer | Blocked | Blocked |
| Approve | Blocked | N/A |
| Revoke | Blocked | N/A |
| Burn | Blocked | N/A |
| MintTo | N/A | Blocked |

#### Freeze/Thaw Operations

- `FreezeAccount`: Changes state from `Initialized` to `Frozen`
- `ThawAccount`: Changes state from `Frozen` to `Initialized`
- Both require the mint's `freeze_authority` signature
- Can override any default state

#### Update

- Changes the default state for future token accounts
- Requires the mint's `freeze_authority` signature
- Cannot set state to `Uninitialized`

### Validation Rules

1. **Cannot set to Uninitialized**: Both Initialize and Update reject `Uninitialized` with `InvalidState`
2. **Freeze authority required for Frozen default**: If default is Frozen, mint must have freeze authority
3. **Update requires freeze authority**: Only freeze authority can change default state
4. **Extension before mint**: Initialize only works on uninitialized mints

---

## 3. PermanentDelegate

### Overview

The PermanentDelegate extension allows a mint authority to designate an address that has permanent, irrevocable transfer and burn authority over all token accounts for that mint. Unlike regular delegates which are per-account and can be revoked by account owners, the permanent delegate operates at the mint level and cannot be removed by token holders.

### Data Structures

#### PermanentDelegate (Mint Extension)

```rust
pub struct PermanentDelegate {
    /// Optional permanent delegate for transferring or burning tokens
    pub delegate: OptionalNonZeroPubkey,
}
```

### Instruction Behavior

#### Transfer

The authority hierarchy for transfers is:

1. **Permanent Delegate (highest priority)**: If signer matches mint's permanent delegate, transfer authorized immediately. No `delegated_amount` consumed.
2. **Regular Delegate**: If signer matches account's delegate, `delegated_amount` is decremented.
3. **Owner (default)**: Account owner must sign.

#### Burn

Same authority hierarchy as Transfer. Permanent delegate can burn without consuming `delegated_amount`.

#### Approve/Revoke

The permanent delegate has **no special privileges**:
- **Approve**: Only account owner can set a delegate
- **Revoke**: Only account owner can revoke delegation

#### SetAuthority (PermanentDelegate type)

The permanent delegate can transfer or renounce their authority. Can be set to `None` to permanently renounce.

### Validation Rules

| Check | Description |
|-------|-------------|
| Extension initialized before mint | Must be called on uninitialized mint |
| Authority signature | Permanent delegate must sign when acting as authority |
| No delegated_amount consumption | Transfers/burns do not affect account's delegated_amount |

### Key Differences from Regular Delegate

| Aspect | Regular Delegate | Permanent Delegate |
|--------|------------------|-------------------|
| Scope | Per-account | Mint-wide (all accounts) |
| Set by | Account owner | Mint authority (at initialization) |
| Revocable | Yes (by owner) | Only by itself (SetAuthority) |
| Amount limit | `delegated_amount` | Unlimited |

---

## 4. TransferHook

### Overview

The TransferHook extension enables mints to specify an external program that gets invoked during token transfers, allowing custom validation logic or side effects to be executed as part of every transfer operation.

### Data Structures

#### TransferHook (Mint Extension)

```rust
pub struct TransferHook {
    /// Authority that can set the transfer hook program id
    pub authority: OptionalNonZeroPubkey,
    /// Program that authorizes the transfer
    pub program_id: OptionalNonZeroPubkey,
}
```

#### TransferHookAccount (Account Extension)

```rust
pub struct TransferHookAccount {
    /// Flag to indicate that the account is in the middle of a transfer
    pub transferring: PodBool,
}
```

A reentrancy guard flag set to `true` during the hook CPI and unset afterward.

### Instruction Behavior

#### Transfer

The transfer hook is invoked **after** balance updates:

1. Source and destination account balances are updated
2. The `transferring` flag is set on both accounts
3. CPI is made to hook program via `spl_transfer_hook_interface::onchain::invoke_execute()`
4. The `transferring` flag is unset

#### Burn / MintTo

Transfer hooks are **not** invoked during burn or mint_to operations.

#### Update

- Requires signature from current `authority`
- Supports multisig authorities
- Can set program_id to `None` to disable the hook

### Validation Rules

1. **Program ID Self-Reference Prevention**: Hook program_id cannot be Token-2022 program itself
2. **Initialization Requirements**: At least one of `authority` or `program_id` must be provided
3. **Reentrancy Protection**: `transferring` flag prevents recursive transfers
4. **Mint Required**: Transfers with `TransferHookAccount` extension must include mint account

---

## 5. Pausable

### Overview

The Pausable extension enables a mint authority to temporarily halt all token movements (transfers, minting, and burning) for an entire token mint. When paused, tokens cannot be moved but other account management operations remain functional.

### Data Structures

#### PausableConfig (Mint Extension)

```rust
pub struct PausableConfig {
    /// Authority that can pause or resume activity on the mint
    pub authority: OptionalNonZeroPubkey,
    /// Whether minting / transferring / burning tokens is paused
    pub paused: PodBool,
}
```

#### PausableAccount (Account Extension)

```rust
pub struct PausableAccount;
```

A zero-sized marker extension added to token accounts belonging to a pausable mint.

### Instruction Behavior

| Instruction | When Paused | Notes |
|-------------|-------------|-------|
| **Transfer** | Blocked | Returns `MintPaused` error |
| **MintTo** | Blocked | Returns `MintPaused` error |
| **Burn** | Blocked | Returns `MintPaused` error |
| **Approve** | Allowed | No pause check performed |
| **Revoke** | Allowed | No pause check performed |
| **FreezeAccount** | Allowed | No pause check performed |
| **ThawAccount** | Allowed | No pause check performed |

### Pause/Resume Instructions

**Pause**:
- Accounts: `[writable] mint`, `[signer] pause_authority`
- Sets `paused = true`
- Supports multisig authority

**Resume**:
- Accounts: `[writable] mint`, `[signer] pause_authority`
- Sets `paused = false`
- Supports multisig authority

### Validation Rules

1. **Authority Validation**: Pause/Resume require authority signature. Returns `AuthorityTypeNotSupported` if authority is `None`.
2. **Transfer Validation**: When paused, returns `TokenError::MintPaused`
3. **Account Extension Enforcement**: Accounts with `PausableAccount` must include mint during transfer
4. **Initialization Order**: Initialize must be called before `InitializeMint`
