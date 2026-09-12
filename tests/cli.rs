use std::process::Command;

fn pget() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pget"))
}

#[test]
fn generate_default_format_is_hex() {
    let output = pget()
        .args(["generate", "256"])
        .output()
        .expect("failed to run pget");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Bits: 256"));
    assert!(stdout.contains("Bytes: 32"));
    assert!(stdout.contains("Format: hex"));

    let hex_line = stdout.lines().last().unwrap();
    assert_eq!(hex_line.len(), 64);
    assert!(hex_line.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn generate_supports_non_byte_aligned_sizes() {
    let output = pget()
        .args(["generate", "13"])
        .output()
        .expect("failed to run pget");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Bits: 13"));
    assert!(stdout.contains("Bytes: 2"));
}

#[test]
fn generate_supports_format_flag() {
    let output = pget()
        .args(["generate", "16", "--format", "binary"])
        .output()
        .expect("failed to run pget");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let bin_line = stdout.lines().last().unwrap();
    assert_eq!(bin_line.len(), 16);
    assert!(bin_line.chars().all(|c| c == '0' || c == '1'));
}

#[test]
fn generate_rejects_duplicate_format_flag() {
    let output = pget()
        .args(["generate", "16", "--format", "hex", "--format", "binary"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("more than once"));
}

#[test]
fn generate_rejects_unexpected_trailing_argument() {
    let output = pget()
        .args(["generate", "16", "junk"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unexpected argument"));
}

#[test]
fn generate_rejects_bits_above_max() {
    let output = pget()
        .args(["generate", "99999999999"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("must not exceed"));
}

#[test]
fn generate_rejects_decimal_above_its_own_cap() {
    // format_decimal is O(n^2); this cap is tighter than MAX_BITS and
    // must be enforced before any bytes are generated.
    let output = pget()
        .args(["generate", "9999999", "--format", "decimal"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--format decimal only supports up to"));
}

#[test]
fn generate_rejects_unknown_format_before_generating_at_large_bits() {
    // An invalid format must be caught before OS entropy is spent,
    // even at a large bit length.
    let output = pget()
        .args(["generate", "9999999", "--format", "octal"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unknown format"));
}

#[test]
fn generate_rejects_unknown_format() {
    let output = pget()
        .args(["generate", "16", "--format", "octal"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unknown format"));
}

#[test]
fn generate_rejects_zero_bits() {
    let output = pget()
        .args(["generate", "0"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("bit length must be greater than zero"));
}

#[test]
fn generate_rejects_negative_bits() {
    let output = pget()
        .args(["generate", "-256"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("bit length must be positive"));
}

#[test]
fn generate_rejects_non_numeric_bits() {
    let output = pget()
        .args(["generate", "abc"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("invalid bit length"));
}

#[test]
fn info_reports_size_metadata() {
    let output = pget()
        .args(["info", "256"])
        .output()
        .expect("failed to run pget");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Bits:             256"));
    assert!(stdout.contains("Bytes:            32"));
    assert!(stdout.contains("Hex characters:   64"));
    assert!(stdout.contains("Possible values:  2^256"));
}

#[test]
fn info_rejects_trailing_argument() {
    let output = pget()
        .args(["info", "256", "junk"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unexpected argument"));
}

#[test]
fn info_rejects_bits_above_max() {
    let output = pget()
        .args(["info", "99999999999"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("must not exceed"));
}

#[test]
fn unknown_command_is_rejected() {
    let output = pget()
        .args(["frobnicate", "256"])
        .output()
        .expect("failed to run pget");
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unknown command"));
}