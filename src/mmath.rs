/// Go `mmath` package port.

/// Absolute value (Go `mmath.Abs`).
pub fn abs(i: i64) -> i64 {
    if i < 0 {
        i * -1
    } else {
        i
    }
}

/// Computes a modulo b.
/// If |a| equals b, 0 is returned.
/// Else if a is negative, b - |a| % b is returned.
pub fn r#mod(a: i64, b: i64) -> i64 {
    (a % b + b) % b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modulo() {
        assert_eq!(0, r#mod(0, 3));
        assert_eq!(0, r#mod(3, 3));
        assert_eq!(4, r#mod(4, 8));
        assert_eq!(0, r#mod(8, 4));
        assert_eq!(2, r#mod(-13, 3));
        assert_eq!(4, r#mod(-10, 7));
    }
}
