pub const NUM_SLOTS_PER_TWO_YEARS: u64 = 157680000;

/// if is_decompress - pay for full delay + 5000 lamports
/// else pay for diff between current slot and last slot so that the full delay is funded again
/// - if the full delay is not funded anymore the account will be compressed
///
/// min top up is 1 lamport
pub fn calculate_top_up(
    _num_bytes: u32,
    last_slot: u64, // last written by somebody else
    current_slot: u64,
    compression_delay: u64,
    is_decompress: bool,
    yield_percentage_points: u64,
) -> u64 {
    let mut base = 0;

    // TODO: copy rent curve
    #[cfg(not(target_os = "solana"))]
    let rent_exemption: u64 = 2630880; // Approx token account with extension
    #[cfg(target_os = "solana")]
    let rent_exemption: u64 = {
        use pinocchio::sysvars::rent::{Rent, SolanaSysvar};
        let rent = Rent::get().unwrap();
        rent.minimum_balance(_num_bytes as usize)
    };

    let slots_to_pay = if is_decompress {
        compression_delay
    } else {
        current_slot.saturating_sub(last_slot)
    };
    if is_decompress || slots_to_pay > compression_delay {
        // Transaction fee to compress the account
        // if slots_to_pay > compression_delay a compression transaction
        // by a forester might fail because of a user transaction
        // the state remains accessible but the cost must be accounted for.
        // Additionally 5000 lamports is equivalent to the zk compression fee to decompress.
        // The slots to pay are uncapped to reward the locked up capital in case that it is still used.
        base += 5000;
    }
    let total_two_year_yield = (rent_exemption * yield_percentage_points) / 10000;
    let yield_fee = ((total_two_year_yield * slots_to_pay) / NUM_SLOTS_PER_TWO_YEARS).max(1);
    base += yield_fee; // Minimum 1 lamport yield_fee
    base
}
