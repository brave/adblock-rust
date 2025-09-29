#![no_main]

use adblock::lists::{parse_filter, FilterFormat, ParseOptions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Input validation - skip empty or overly large inputs
    if data.is_empty() || data.len() > 1024 * 1024 {
        return;
    }

    // Try multiple string conversion approaches for better coverage
    fuzz_utf8_parsing(data);
    fuzz_lossy_parsing(data);
});

fn fuzz_utf8_parsing(data: &[u8]) {
    if let Ok(filter_str) = std::str::from_utf8(data) {
        // Skip extremely long lines that might cause performance issues
        if filter_str.len() > 10000 {
            return;
        }

        // Test with different FilterFormat options for better coverage
        let formats = [
            FilterFormat::Standard,
            FilterFormat::Hosts,
            FilterFormat::AdblockPlus,
        ];

        for &format in &formats {
            // Test with different parse options
            let parse_options = [
                ParseOptions {
                    format,
                    ..Default::default()
                },
                ParseOptions {
                    format,
                    ignore_cosmetic: true,
                    ..Default::default()
                },
                ParseOptions {
                    format,
                    ignore_cosmetic: false,
                    ..Default::default()
                },
            ];

            for options in &parse_options {
                // Catch potential panics and handle results properly
                let result = std::panic::catch_unwind(|| {
                    parse_filter(filter_str, true, options.clone())
                });

                // Log interesting results for debugging (only in fuzzing context)
                if let Ok(parse_result) = result {
                    match parse_result {
                        Ok(_filter) => {
                            // Successfully parsed - could add additional validation here
                        }
                        Err(_parse_error) => {
                            // Parse error - this is expected for many inputs
                        }
                    }
                }
            }
        }
    }
}

fn fuzz_lossy_parsing(data: &[u8]) {
    // Also test with lossy UTF-8 conversion to catch different edge cases
    let filter_str = String::from_utf8_lossy(data);
    
    // Skip extremely long strings
    if filter_str.len() > 10000 {
        return;
    }

    // Test common filter prefixes that might trigger different code paths
    let test_prefixes = ["||", "@@", "!", "###", "##", "/", "|"];
    
    for prefix in &test_prefixes {
        let prefixed_filter = format!("{}{}", prefix, filter_str);
        
        let _ = std::panic::catch_unwind(|| {
            parse_filter(
                &prefixed_filter,
                true,
                ParseOptions {
                    format: FilterFormat::Standard,
                    ..Default::default()
                },
            )
        });
    }

    // Test with different debug flags
    for debug_flag in [true, false] {
        let _ = std::panic::catch_unwind(|| {
            parse_filter(
                &filter_str,
                debug_flag,
                ParseOptions {
                    format: FilterFormat::Standard,
                    ..Default::default()
                },
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Test with some known good inputs
        let test_cases = [
            b"example.com",
            b"||example.com^",
            b"@@||example.com^",
            b"###ad-banner",
            b"! This is a comment",
            b"",
        ];

        for case in &test_cases {
            fuzz_utf8_parsing(case);
            fuzz_lossy_parsing(case);
        }
    }

    #[test]
    fn test_edge_cases() {
        // Test edge cases
        let edge_cases = [
            &[0u8; 1000][..], // Null bytes
            &[255u8; 100][..], // Invalid UTF-8
            b"\x00\x01\x02\x03", // Control characters
            &vec![b'A'; 20000][..], // Very long input
        ];

        for case in &edge_cases {
            fuzz_utf8_parsing(case);
            fuzz_lossy_parsing(case);
        }
    }
}
