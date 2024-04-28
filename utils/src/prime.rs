pub fn find_next_prime(mut n: f64) -> f64 {
    n = n.round();

    // Handle small numbers separately
    if n <= 2.0 {
        return 2.0;
    } else if n <= 3.0 {
        return 3.0;
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
    let remainder = n % 6.0;
    if remainder != 0.0 {
        n = n + 6.0 - remainder;

        let candidate = n - 1.0;
        if is_prime(candidate) {
            return candidate;
        }
    }

    loop {
        let candidate = n + 1.0;
        if is_prime(candidate) {
            return candidate;
        }
        let candidate = n + 5.0;
        if is_prime(candidate) {
            return candidate;
        }

        n += 6.0;
    }
}

pub fn is_prime(n: f64) -> bool {
    if n <= 1.0 {
        return false;
    }
    if n <= 3.0 {
        return true;
    }
    if n % 2.0 == 0.0 || n % 3.0 == 0.0 {
        return false;
    }
    let mut i = 5.0;
    while i * i <= n {
        if n % i == 0.0 || n % (i + 2.0) == 0.0 {
            return false;
        }
        i += 6.0;
    }
    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_next_prime() {
        assert_eq!(find_next_prime(0.0), 2.0);
        assert_eq!(find_next_prime(2.0), 2.0);
        assert_eq!(find_next_prime(3.0), 3.0);
        assert_eq!(find_next_prime(4.0), 5.0);

        assert_eq!(find_next_prime(10.0), 11.0);
        assert_eq!(find_next_prime(28.0), 29.0);

        assert_eq!(find_next_prime(100.0), 101.0);
        assert_eq!(find_next_prime(1000.0), 1009.0);

        assert_eq!(find_next_prime(102.0), 103.0);
        assert_eq!(find_next_prime(105.0), 107.0);

        assert_eq!(find_next_prime(7900.0), 7901.0);
        assert_eq!(find_next_prime(7907.0), 7907.0);
    }
}
