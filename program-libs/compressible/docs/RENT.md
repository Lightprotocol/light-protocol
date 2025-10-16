# Rent Calculation APIs

## Description
Rent calculation functions determine when compressible ctoken accounts can be compressed or paid rent be claimed.

**Key Concepts:**
- **Rent Epochs:** Rent is calculated per epoch (432,000 slots). Accounts must maintain sufficient balance for all epochs since last claim.
- **Compression Incentive:** Accounts receive `COMPRESSION_COST + COMPRESSION_INCENTIVE` when created to cover future compression.
- **Partial Epochs:** When closing, partial epoch rent is returned to the user, completed epochs go to rent recipient.
- **Compressibility Window:** Accounts become compressible when they lack rent for the current epoch + 1.

## Constants

```rust
pub const COMPRESSION_COST: u16 = 10_000;       // Base compression operation cost (5000 lamports compression fee + 5000 lamports forester tx fee)
pub const COMPRESSION_INCENTIVE: u16 = 1000;    // Incentive for compression for the forester node
pub const BASE_RENT: u16 = 1220;                 // Minimum rent per epoch
pub const RENT_PER_BYTE: u8 = 10;               // Rent per byte per epoch
pub const SLOTS_PER_EPOCH: u64 = 432_000;       // Solana slots per epoch
```

## RentConfig

**Path:** `program-libs/compressible/src/rent.rs`

**Size:** 8 bytes

```rust
pub struct RentConfig {
    pub base_rent: u16,                   // Minimum rent per epoch
    pub compression_cost: u16, // Total compression cost + incentive
    pub lamports_per_byte_per_epoch: u8,               // Rent per byte per epoch
    _place_holder_bytes: [u8; 3],       // Padding
}
```

### RentConfig Methods

- `rent_curve_per_epoch(num_bytes)` - Calculate rent for given bytes per epoch
- `get_rent(num_bytes, epochs)` - Calculate total rent for multiple epochs
- `get_rent_with_compression_cost(num_bytes, epochs)` - Rent plus compression costs

## Core Functions

### Rent Calculation

#### `rent_curve_per_epoch`
```rust
pub fn rent_curve_per_epoch(base_rent: u64, lamports_per_byte_per_epoch: u64, num_bytes: u64) -> u64
```
Calculates rent required per epoch for an account of given size.

**Formula:** `base_rent + (num_bytes * lamports_per_byte_per_epoch)`

**Use:** Base calculation for all rent operations

---

#### `get_rent`
```rust
pub fn get_rent(base_rent: u64, lamports_per_byte_per_epoch: u64, num_bytes: u64, epochs: u64) -> u64
```
Calculates total rent for multiple epochs.

**Formula:** `rent_curve_per_epoch * epochs`

**Use:** Calculate rent requirements for future epochs

---

#### `get_rent_exemption_lamports`
```rust
pub fn get_rent_exemption_lamports(num_bytes: u64) -> Result<u64, CompressibleError>
```
Returns Solana's rent-exempt balance for account size.

**Returns:** Minimum lamports to keep account rent-exempt

**Error:** `FailedBorrowRentSysvar` if rent sysvar unavailable

### Compressibility Check

#### `calculate_rent_and_balance`
```rust
pub fn calculate_rent_and_balance(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> (bool, u64)
```
Determines if an account is compressible and calculates deficit if needed.

**Returns:** `(is_compressible, lamports_deficit)`
- `true, deficit` - Account can be compressed, needs `deficit` lamports
- `false, 0` - Account has sufficient rent, not compressible

**Logic:**
1. Calculates epochs since last claim
2. Determines required rent for those epochs
3. Checks if account balance covers required rent
4. Returns compressibility status and any deficit

**Use:** Primary check before compression operations

### Rent Claims

#### `claimable_lamports`
```rust
pub fn claimable_lamports(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> Option<u64>
```
Calculates rent that can be claimed from a funded account.

**Returns:**
- `Some(amount)` - Claimable rent for completed epochs
- `None` - Account is compressible (should compress, not claim)

**Logic:**
1. First checks if account is compressible
2. If not compressible, calculates rent for completed epochs only
3. Current ongoing epoch rent cannot be claimed

**Use:** Determine claimable amount in `Claim` instruction

### Close Account Distribution

#### `calculate_close_lamports`
```rust
pub fn calculate_close_lamports(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> (u64, u64)
```
Splits account lamports between rent recipient and user on close.

**Returns:** `(lamports_to_rent_sponsor, lamports_to_user)`

**Logic:**
1. Calculates unutilized rent (partial epoch remainder)
2. Rent recipient gets: total - unutilized
3. User gets: unutilized lamports

**Use:** Distribute funds when closing compressible accounts

### Helper Functions

#### `calculate_rent_inner`
```rust
pub fn calculate_rent_inner<const INCLUDE_CURRENT: bool>(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> (u64, u64, u64, u64)
```
Internal calculation function for rent analysis.

**Returns:** `(required_epochs, rent_per_epoch, epochs_paid, unutilized_lamports)`
- `required_epochs` - Total epochs needing rent (includes current if INCLUDE_CURRENT=true)
- `rent_per_epoch` - Rent amount per epoch
- `epochs_paid` - Number of epochs covered by available balance
- `unutilized_lamports` - Partial epoch rent that cannot be claimed

**Use:** Low-level rent calculations used by other functions

---

#### `get_last_funded_epoch`
```rust
pub fn get_last_funded_epoch(
    num_bytes: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> u64
```
Determines the last epoch covered by rent payments.

**Returns:** Last epoch number with paid rent

**Use:** Track rent payment status

## Usage Examples

### Check if account is compressible
```rust
let (is_compressible, deficit) = calculate_rent_and_balance(
    260,        // account size
    1000000,    // current slot
    5000000,    // current lamports
    0,          // last claimed slot
    2000000,    // rent exempt amount
    1220,       // min rent
    10,         // rent per byte
    11000,      // compression incentive
);
```

### Calculate claimable rent
```rust
let claimable = claimable_lamports(
    260, 1000000, 5000000, 0, 2000000, 1220, 10, 11000
);
// Returns Some(amount) if claimable, None if compressible
```

### Split lamports on close
```rust
let (to_rent_sponsor, to_user) = calculate_close_lamports(
    260, 1000000, 5000000, 0, 2000000, 1220, 10, 11000
);
```
