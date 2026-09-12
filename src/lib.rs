use rand::rngs::OsRng;
use rand::RngCore;

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Upper bound on bit length. Without it, a mistyped or malicious value
/// makes `generate_bytes` try to allocate hundreds of megabytes.
/// 2^30 bits = 128 MiB, far beyond any real key/nonce size.
pub const MAX_BITS: u64 = 1 << 30;

/// Rejects a bit length of zero or above `MAX_BITS`.
pub fn validate_bits(bits: u64) -> Result<(), String> {
    if bits == 0 {
        return Err("bit length must be greater than zero".to_string());
    }
    if bits > MAX_BITS {
        return Err(format!(
            "bit length must not exceed {} (got {})",
            MAX_BITS, bits
        ));
    }
    Ok(())
}

/// Number of bytes needed to hold `bits` bits. Saturating so a `bits`
/// value near `u64::MAX` returns `usize::MAX` bytes instead of
/// panicking or wrapping. `generate_bytes` never hits that edge, since
/// `validate_bits` rejects anything above `MAX_BITS` first.
pub fn byte_length(bits: u64) -> usize {
    (bits.saturating_add(7) / 8) as usize
}

/// Clears the unused high bits in the final byte of a big-endian
/// buffer, so only the requested number of bits is significant.
///
/// Defensive against misuse: if `bits` doesn't match `data`'s actual
/// capacity (more bits requested than the buffer holds, or far fewer),
/// this clamps rather than underflowing/panicking. Callers going
/// through `generate_bytes` always pass a buffer sized by
/// `byte_length(bits)`, so this only matters for direct callers.
pub fn mask_unused_bits(data: &mut [u8], bits: u64) {
    let total_bits = (data.len() as u64) * 8;
    let extra_bits = total_bits.saturating_sub(bits);
    if extra_bits == 0 {
        return;
    }
    let Some(last) = data.last_mut() else {
        return;
    };
    if extra_bits >= 8 {
        *last = 0;
        return;
    }
    *last &= 0xFFu8 >> extra_bits;
}

/// Generates `bits` bits of cryptographically secure random data from
/// the operating system's CSPRNG, with unused high bits cleared.
pub fn generate_bytes(bits: u64) -> Result<Vec<u8>, String> {
    validate_bits(bits)?;
    let mut buf = vec![0u8; byte_length(bits)];
    OsRng
        .try_fill_bytes(&mut buf)
        .map_err(|e| format!("failed to read from the OS random number generator: {}", e))?;
    mask_unused_bits(&mut buf, bits);
    Ok(buf)
}

pub fn generate_hex(bits: u64) -> Result<String, String> {
    generate_bytes(bits).map(|b| format_hex(&b))
}

pub fn generate_binary(bits: u64) -> Result<String, String> {
    generate_bytes(bits).map(|b| format_binary(&b))
}

pub fn generate_decimal(bits: u64) -> Result<String, String> {
    generate_bytes(bits).map(|b| format_decimal(&b))
}

pub fn generate_base64(bits: u64) -> Result<String, String> {
    generate_bytes(bits).map(|b| format_base64(&b))
}

pub fn format_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn format_binary(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:08b}", b)).collect()
}

/// Converts a big-endian byte buffer into a base-10 string by
/// repeatedly dividing the buffer by 10^9 and collecting 9-digit
/// decimal groups (rather than dividing by 10 one digit at a time).
/// That cuts the number of outer-loop passes ~9x, but each pass is
/// still O(len), so total cost is still quadratic in `data.len()`.
/// For very large buffers this can take a long time — callers exposing
/// this to untrusted sizes should cap the input length before calling
/// it (see `pget`'s CLI, which limits `--format decimal` separately
/// from the general `MAX_BITS` cap).
pub fn format_decimal(data: &[u8]) -> String {
    if data.iter().all(|&b| b == 0) {
        return "0".to_string();
    }

    const CHUNK: u64 = 1_000_000_000; // 10^9

    let mut num = data.to_vec();
    let mut groups: Vec<u32> = Vec::new(); // base-10^9 groups, least-significant first

    while !num.iter().all(|&b| b == 0) {
        let mut remainder: u64 = 0;
        for byte in num.iter_mut() {
            let cur = (remainder << 8) | (*byte as u64);
            *byte = (cur / CHUNK) as u8;
            remainder = cur % CHUNK;
        }
        groups.push(remainder as u32);
    }

    let mut out = String::new();
    for (i, group) in groups.iter().rev().enumerate() {
        if i == 0 {
            out.push_str(&group.to_string());
        } else {
            out.push_str(&format!("{:09}", group));
        }
    }
    out
}

pub fn format_base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        out.push(BASE64_CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(BASE64_CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            BASE64_CHARS[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64_CHARS[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_bits_rejects_zero() {
        assert!(validate_bits(0).is_err());
        assert!(validate_bits(1).is_ok());
    }

    #[test]
    fn validate_bits_rejects_above_max() {
        assert!(validate_bits(MAX_BITS).is_ok());
        assert!(validate_bits(MAX_BITS + 1).is_err());
        assert!(validate_bits(u64::MAX).is_err());
    }

    #[test]
    fn byte_length_rounds_up() {
        assert_eq!(byte_length(1), 1);
        assert_eq!(byte_length(8), 1);
        assert_eq!(byte_length(9), 2);
        assert_eq!(byte_length(256), 32);
        assert_eq!(byte_length(4096), 512);
    }

    #[test]
    fn byte_length_does_not_overflow_near_u64_max() {
        // Regression test: (bits + 7) without saturation panics in
        // debug builds and wraps in release builds for bits close to
        // u64::MAX. This must not panic.
        assert_eq!(byte_length(u64::MAX), (u64::MAX / 8) as usize);
    }

    #[test]
    fn generate_bytes_rejects_bits_above_max() {
        assert!(generate_bytes(MAX_BITS + 1).is_err());
    }

    #[test]
    fn mask_unused_bits_clears_high_bits_of_last_byte() {
        let mut buf = vec![0xFF, 0xFF];
        mask_unused_bits(&mut buf, 13);
        assert_eq!(buf, vec![0xFF, 0x1F]);
    }

    #[test]
    fn mask_unused_bits_noop_when_byte_aligned() {
        let mut buf = vec![0xFF, 0xFF];
        mask_unused_bits(&mut buf, 16);
        assert_eq!(buf, vec![0xFF, 0xFF]);
    }

    #[test]
    fn mask_unused_bits_does_not_panic_when_bits_exceeds_buffer() {
        // Regression test: a misused/direct call with `bits` larger
        // than the buffer previously underflowed `total_bits - bits`.
        let mut buf = vec![0xFF];
        mask_unused_bits(&mut buf, 100);
        assert_eq!(buf, vec![0xFF]);
    }

    #[test]
    fn mask_unused_bits_does_not_panic_when_bits_much_smaller_than_buffer() {
        // Regression test: `extra_bits >= 8` previously overflowed the
        // `0xFF >> extra_bits` shift (extra_bits didn't fit in a u8 shift).
        let mut buf = vec![0xFF, 0xFF, 0xFF];
        mask_unused_bits(&mut buf, 3);
        assert_eq!(buf, vec![0xFF, 0xFF, 0x00]);
    }

    #[test]
    fn mask_unused_bits_empty_buffer_does_not_panic() {
        let mut buf: Vec<u8> = vec![];
        mask_unused_bits(&mut buf, 5);
        assert_eq!(buf, Vec::<u8>::new());
    }

    #[test]
    fn generate_bytes_respects_length_and_rejects_zero() {
        assert_eq!(generate_bytes(256).unwrap().len(), 32);
        assert_eq!(generate_bytes(13).unwrap().len(), 2);
        assert!(generate_bytes(0).is_err());
    }

    #[test]
    fn format_hex_is_correct() {
        assert_eq!(format_hex(&[0x9f, 0x31, 0xc7]), "9f31c7");
    }

    #[test]
    fn format_binary_is_correct() {
        assert_eq!(format_binary(&[0b10110001]), "10110001");
    }

    #[test]
    fn format_decimal_matches_known_values() {
        assert_eq!(format_decimal(&[0x00]), "0");
        assert_eq!(format_decimal(&[0xFF]), "255");
        assert_eq!(format_decimal(&[0x01, 0x00]), "256");
    }

    #[test]
    fn format_decimal_spans_multiple_10e9_groups() {
        // 2^32, which needs two base-10^9 groups: exercises the
        // group-boundary logic (leading group unpadded, rest zero-padded).
        assert_eq!(
            format_decimal(&[0x01, 0x00, 0x00, 0x00, 0x00]),
            "4294967296"
        );
        // 10^9 + 1: low group is "000000001", not "1".
        assert_eq!(format_decimal(&[0x3B, 0x9A, 0xCA, 0x01]), "1000000001");
    }

    #[test]
    fn format_base64_matches_known_values() {
        // "Man" -> "TWFu" is the classic base64 test vector.
        assert_eq!(format_base64(b"Man"), "TWFu");
        assert_eq!(format_base64(b"Ma"), "TWE=");
        assert_eq!(format_base64(b"M"), "TQ==");
    }
}
