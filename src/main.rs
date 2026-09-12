use pget::{
    byte_length, format_base64, format_binary, format_decimal, format_hex, generate_bytes,
    validate_bits,
};
use std::env;
use std::process;

const FORMATS: &[&str] = &["hex", "binary", "decimal", "base64"];

/// `format_decimal`'s conversion is O(n^2) in the byte length, so
/// letting `--format decimal` run all the way up to `pget::MAX_BITS`
/// (128 MiB of output) would let an otherwise-valid `generate` call
/// hang indefinitely. This is a much tighter, decimal-specific cap;
/// even at the limit, conversion stays well under a second.
const MAX_DECIMAL_BITS: u64 = 1 << 17; // 131,072 bits = 16 KiB

fn parse_bits(arg: &str) -> Result<u64, String> {
    match arg.parse::<i128>() {
        Ok(0) => Err("bit length must be greater than zero".to_string()),
        Ok(n) if n < 0 => Err("bit length must be positive".to_string()),
        Ok(n) if n > u64::MAX as i128 => Err("bit length is too large".to_string()),
        Ok(n) => Ok(n as u64),
        Err(_) => Err("invalid bit length".to_string()),
    }
}

/// Parses `[--format <value>]` from the arguments following `<bits>`.
/// Rejects anything it doesn't recognize (duplicate flag, stray
/// positional argument) instead of silently ignoring it.
fn parse_format_flag(args: &[String]) -> Result<String, String> {
    let mut format: Option<String> = None;
    let mut iter = args.iter();

    while let Some(token) = iter.next() {
        match token.as_str() {
            "--format" => {
                if format.is_some() {
                    return Err("--format specified more than once".to_string());
                }
                let value = iter
                    .next()
                    .ok_or_else(|| "--format requires a value".to_string())?;
                format = Some(value.clone());
            }
            other => return Err(format!("unexpected argument '{}'", other)),
        }
    }

    Ok(format.unwrap_or_else(|| "hex".to_string()))
}

fn validate_format(format: &str) -> Result<(), String> {
    if FORMATS.contains(&format) {
        Ok(())
    } else {
        Err(format!(
            "unknown format '{}' (expected hex, binary, decimal, or base64)",
            format
        ))
    }
}

fn format_value(data: &[u8], format: &str) -> Result<String, String> {
    match format {
        "hex" => Ok(format_hex(data)),
        "binary" => Ok(format_binary(data)),
        "decimal" => Ok(format_decimal(data)),
        "base64" => Ok(format_base64(data)),
        other => Err(format!(
            "unknown format '{}' (expected hex, binary, decimal, or base64)",
            other
        )),
    }
}

fn cmd_generate(args: &[String]) -> Result<(), String> {
    let bits_arg = args.first().ok_or("missing bit length")?;
    let bits = parse_bits(bits_arg)?;
    let format = parse_format_flag(&args[1..])?;
    validate_format(&format)?;

    if format == "decimal" && bits > MAX_DECIMAL_BITS {
        return Err(format!(
            "--format decimal only supports up to {} bits (got {}); \
             use hex, binary, or base64 for larger values",
            MAX_DECIMAL_BITS, bits
        ));
    }

    let data = generate_bytes(bits)?;
    let value = format_value(&data, &format)?;

    println!("Bits: {}", bits);
    println!("Bytes: {}", data.len());
    println!("Format: {}", format);
    println!();
    println!("{}", value);
    Ok(())
}

fn cmd_info(args: &[String]) -> Result<(), String> {
    let bits_arg = args.first().ok_or("missing bit length")?;
    if let Some(extra) = args.get(1) {
        return Err(format!("unexpected argument '{}'", extra));
    }
    let bits = parse_bits(bits_arg)?;
    validate_bits(bits)?;
    let bytes = byte_length(bits);

    println!("Bits:             {}", bits);
    println!("Bytes:            {}", bytes);
    println!("Hex characters:   {}", bytes * 2);
    println!("Possible values:  2^{}", bits);
    Ok(())
}

fn print_help() {
    println!("pget — generate cryptographically secure random values");
    println!();
    println!("USAGE:");
    println!("    pget generate <bits> [--format hex|binary|decimal|base64]");
    println!("    pget info <bits>");
    println!("    pget --help");
    println!("    pget --version");
    println!();
    println!("NOTE:");
    println!(
        "    --format decimal is limited to {} bits (conversion cost grows quadratically).",
        MAX_DECIMAL_BITS
    );
}

fn print_version() {
    println!("pget {}", env!("CARGO_PKG_VERSION"));
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let result = match args.first().map(String::as_str) {
        Some("--help") | Some("-h") => {
            print_help();
            return;
        }
        Some("--version") | Some("-V") => {
            print_version();
            return;
        }
        Some("generate") => cmd_generate(&args[1..]),
        Some("info") => cmd_info(&args[1..]),
        Some(other) => Err(format!(
            "unknown command '{}' (expected 'generate' or 'info')",
            other
        )),
        None => {
            print_help();
            process::exit(1);
        }
    };

    if let Err(message) = result {
        eprintln!("Error: {}", message);
        process::exit(1);
    }
}
