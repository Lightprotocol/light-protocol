/// Finds the lowest prime number which is greater than the provided number
/// `n`.
pub fn find_next_prime(mut n: u32) -> u32 {
    // Handle small numbers separately
    if n <= 2 {
        return 2;
    } else if n <= 3 {
        return 3;
    }

    // All prime numbers greater than 3 are of the form 6k + 1 or 6k + 5 (or
    // 6k - 1).
    // That's because:
    //
    // 6k is divisible by 2 and 3.
    // 6k + 2 = 2(3k + 1) is divisible by 2.
    // 6k + 3 = 3(2k + 1) is divisible by 3.
    // 6k + 4 = 2(3k + 2) is divisible by 2.
    //
    // This leaves only 6k + 1 and 6k + 5 as candidates.

    // Ensure the candidate is of the form 6k - 1 or 6k + 1.
    let remainder = n % 6;
    if remainder != 0 {
        // Check if `n` already satisfies the pattern and is prime.
        if remainder == 5 && is_prime(n) {
            return n;
        }
        if remainder == 1 && is_prime(n) {
            return n;
        }

        // Add `6 - remainder` to `n`, to it satisfies the `6k` pattern.
        n = n + 6 - remainder;
        // Check if `6k - 1` candidate is prime.
        let candidate = n - 1;
        if is_prime(candidate) {
            return candidate;
        }
    }

    // Consequently add `6`, keep checking `6k + 1` and `6k + 5` candidates.
    loop {
        let candidate = n + 1;
        if is_prime(candidate) {
            return candidate;
        }
        let candidate = n + 5;
        if is_prime(candidate) {
            return candidate;
        }

        n += 6;
    }
}

pub fn find_next_prime_with_load_factor(n: u32, load_factor: f64) -> u32 {
    // SAFETY: These type coercions should not cause any issues.
    //
    // * `f64` can precisely represent all integer values up to 2^53, which is
    //   more than `u32::MAX`. `u64` and `usize` would be too large though.
    // * We want to return and find an integer (prime number), so coercing `f64`
    //   back to `u32` is intentional here.
    let minimum = n as f64 / load_factor;
    find_next_prime(minimum as u32)
}

/// Checks whether the provided number `n` is a prime number.
pub fn is_prime(n: u32) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    let mut i = 5;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_next_prime() {
        assert_eq!(find_next_prime(0), 2);
        assert_eq!(find_next_prime(2), 2);
        assert_eq!(find_next_prime(3), 3);
        assert_eq!(find_next_prime(4), 5);

        assert_eq!(find_next_prime(10), 11);
        assert_eq!(find_next_prime(17), 17);
        assert_eq!(find_next_prime(19), 19);
        assert_eq!(find_next_prime(28), 29);

        assert_eq!(find_next_prime(100), 101);
        assert_eq!(find_next_prime(102), 103);
        assert_eq!(find_next_prime(105), 107);

        assert_eq!(find_next_prime(1000), 1009);
        assert_eq!(find_next_prime(2000), 2003);
        assert_eq!(find_next_prime(3000), 3001);
        assert_eq!(find_next_prime(4000), 4001);

        assert_eq!(find_next_prime(4800), 4801);
        assert_eq!(find_next_prime(5000), 5003);
        assert_eq!(find_next_prime(6000), 6007);
        assert_eq!(find_next_prime(6850), 6857);

        assert_eq!(find_next_prime(7000), 7001);
        assert_eq!(find_next_prime(7900), 7901);
        assert_eq!(find_next_prime(7907), 7907);
    }

    #[test]
    fn test_find_next_prime_with_load_factor() {
        assert_eq!(find_next_prime_with_load_factor(4800, 0.5), 9601);
        assert_eq!(find_next_prime_with_load_factor(4800, 0.7), 6857);
    }

    #[test]
    fn test_is_prime() {
        assert_eq!(is_prime(1), false);
        assert_eq!(is_prime(2), true);
        assert_eq!(is_prime(3), true);
        assert_eq!(is_prime(4), false);
        assert_eq!(is_prime(17), true);
        assert_eq!(is_prime(19), true);
    }
}
