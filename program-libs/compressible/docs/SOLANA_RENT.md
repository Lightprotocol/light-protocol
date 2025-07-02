# Solana Rent vs Light Protocol Rent

## Overview

This document explains the differences between Solana's native rent system and Light Protocol's rent implementation for compressible CToken accounts.

## Solana Native Rent

### Core Concepts

**Rent Exemption:** Solana accounts must maintain a minimum balance to be rent-exempt, calculated as:
```
minimum_balance = (ACCOUNT_STORAGE_OVERHEAD + data_len) * lamports_per_byte * exemption_threshold
```

**Key Constants:**
- `ACCOUNT_STORAGE_OVERHEAD`: 128 bytes (metadata overhead)
- `DEFAULT_LAMPORTS_PER_BYTE`: 6960 lamports (SIMD-0194)
- `DEFAULT_EXEMPTION_THRESHOLD`: 2.0 years
- `DEFAULT_BURN_PERCENT`: 50% of collected rent is burned

**Rent Collection:** Non-exempt accounts are charged rent based on:
- Account size (data + 128 bytes overhead)
- Time elapsed (in years)
- Rental rate (lamports per byte-year)

## Light Protocol Rent

### Design Philosophy

Light Protocol's rent system is designed for **compressible token accounts** with different goals:
- Incentivize compression when accounts run out of rent
- Distribute rent to protocol participants (not burn)
- Use epoch-based accounting (not continuous time)

### Key Differences

| Aspect | Solana Rent | Light Protocol Rent |
|--------|-------------|-------------------|
| **Time Unit** | Years (continuous) | Epochs (discrete, 432,000 slots) |
| **Rent Rate** | ~6960 lamports/byte for 2 years exemption | 1220 min + 10/byte per epoch |
| **Exemption** | Permanent with sufficient balance | Temporary, epoch-by-epoch |
| **Collection** | Automatic by runtime | Manual via Claim instruction |
| **Distribution** | 50% burned, 50% to validators | 100% to rent recipient (protocol) |
| **Rent-Specific Data** | None (uses account balance) | 88 bytes (CompressionInfo) |
| **Compression** | N/A | Incentivized with 11,000 lamport bonus |

### Rent Calculation Comparison

**Solana (rent-exempt for 100 bytes):**
```rust
// Using Solana's Rent sysvar
let rent = Rent::get()?;
let minimum_balance = rent.minimum_balance(100);
// Result: (128 + 100) * 6960 = 1,586,880 lamports
```

**Light Protocol (rent for 100 bytes):**
```rust
// Per epoch rent
let rent_per_epoch = rent_curve_per_epoch(1220, 10, 100);
// Result: 1220 + (100 * 10) = 2220 lamports per epoch

// To maintain for ~2 years (1051 epochs)
let two_year_rent = 2220 * 1051 = 2,333,220 lamports
```

### Compressibility Window

Unlike Solana's binary rent-exempt status, Light Protocol uses a **compressibility window**:

1. **Funded:** Account has rent for current epoch + 1
2. **Compressible:** Account lacks rent for current epoch + 1
3. **Claimable:** Account funded but past epochs unclaimed

This creates economic incentives:
- Users fund accounts minimally (just enough epochs)
- Protocol can compress inactive accounts
- Rent flows to protocol treasury, not burned

### Integration with Solana

Light Protocol accounts still interact with Solana's rent system:

1. **Base Rent Exemption:** CToken accounts need Solana rent exemption
   - Retrieved via `get_rent_exemption_lamports()`
   - Uses Solana's Rent sysvar internally
   - Subtracted from available balance for Light rent calculations

2. **Account Creation:** Must satisfy both:
   - Solana rent exemption (base lamports)
   - Light Protocol rent (additional lamports)
   - Compression incentive (11,000 lamports)

3. **Account Closure:** Lamports distributed as:
   - Solana rent exemption → returned to user
   - Completed epoch rent → rent recipient
   - Partial epoch rent → user
   - Compression incentive → forester node (when compressed)

## Implementation Details

### Rent Sysvar Access

Light Protocol accesses Solana's Rent sysvar to determine base exemption:

```rust
// program-libs/compressible/src/rent.rs
pub fn get_rent_exemption_lamports(_num_bytes: u64) -> Result<u64, CompressibleError> {
    #[cfg(target_os = "solana")]
    {
        use solana_program::rent::Rent;
        use solana_program::sysvar::Sysvar;

        let rent = Rent::get()
            .map_err(|_| CompressibleError::FailedBorrowRentSysvar)?;
        Ok(rent.minimum_balance(_num_bytes as usize))
    }
    #[cfg(not(target_os = "solana"))]
    {
        // Test environment mock
        Ok(2_282_880)
    }
}
```

### Epoch vs Year Conversion

- **Solana:** Uses floating-point years (deprecated in SIMD-0194)
- **Light:** Uses integer epochs (432,000 slots = ~2.5 days)
- **Conversion:** ~1051 epochs ≈ 2 years

## Summary

Light Protocol's rent system is a **layer on top** of Solana's rent that:
- Requires Solana rent exemption as a base
- Adds epoch-based rent for protocol sustainability
- Incentivizes compression of inactive accounts
- Distributes rent to protocol instead of burning

This dual-rent model ensures accounts remain valid on Solana while enabling Light Protocol's compression economics.
