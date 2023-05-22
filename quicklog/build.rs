use std::env;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;

fn parse_value_from_config_with_default<T: FromStr>(
    key: &str,
    default: Option<T>,
) -> Result<T, String> {
    // Retrieve the value of the environment variable
    match env::var(key) {
        Ok(value) => match value.parse::<T>() {
            Ok(val) => Ok(val),
            Err(_) => {
                Err(format!("env var '{}' with value '{}' cannot be parsed into type '{2}'. Please set an env var can be parsed into '{2}'", key, value, stringify!(T)))
            }
        },
        Err(_) => match default {
            Some(val) => Ok(val),
            None => {
                eprintln!("MAX_LOGGER_CAPACITY environment variable is not set");
                Err(format!("env '{}' is not set and there are no defaults for it. Please set it in your env.", key))
            }
        },
    }
}

fn main() {
    let max_buffer_capacity = match parse_value_from_config_with_default(
        "QUICKLOG_MAX_SERIALIZE_BUFFER_CAPACITY",
        Some(1_000_000_usize),
    ) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    let max_logger_capacity = match parse_value_from_config_with_default(
        "QUICKLOG_MAX_LOGGER_CAPACITY",
        Some(1_000_000_usize),
    ) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    // Generate the Rust source code
    let rust_code = format!(
        "pub const MAX_LOGGER_CAPACITY: usize = {};
pub const MAX_SERIALIZE_BUFFER_CAPACITY: usize = {};
",
        max_logger_capacity, max_buffer_capacity
    );

    // Write the code to a file
    let dest_path = std::path::Path::new("").join("src/constants.rs");
    let mut file = File::create(dest_path).expect("Failed to create file");
    file.write_all(rust_code.as_bytes())
        .expect("Failed to write file");

    println!("cargo:rerun-if-env-changed=MAX_CAPACITY");
}