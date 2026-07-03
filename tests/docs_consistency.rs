/// Integration test for docs consistency
/// Verifies that man page versions match the Cargo.toml version.
/// This ensures documentation doesn't drift from the actual package version.

#[test]
fn man_page_versions_match_cargo_toml() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let pkg_version = env!("CARGO_PKG_VERSION");

    // Read checksmix.1
    let checksmix_man = std::fs::read_to_string(format!("{}/man/checksmix.1", manifest_dir))
        .expect("Could not read man/checksmix.1");
    let checksmix_version =
        extract_th_version(&checksmix_man).expect("Could not extract version from checksmix.1");

    // Read mmixasm.1
    let mmixasm_man = std::fs::read_to_string(format!("{}/man/mmixasm.1", manifest_dir))
        .expect("Could not read man/mmixasm.1");
    let mmixasm_version =
        extract_th_version(&mmixasm_man).expect("Could not extract version from mmixasm.1");

    assert_eq!(
        checksmix_version, pkg_version,
        "man/checksmix.1 version mismatch: expected {}, got {}",
        pkg_version, checksmix_version
    );
    assert_eq!(
        mmixasm_version, pkg_version,
        "man/mmixasm.1 version mismatch: expected {}, got {}",
        pkg_version, mmixasm_version
    );
}

/// Extract version string from man page .TH line
/// .TH lines are formatted as: .TH NAME SECTION DATE "name version"
/// We extract the version from the quoted field after the name.
fn extract_th_version(content: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with(".TH") {
            // Parse: .TH MMIXASM 1 "May 2025" "checksmix 0.2.23"
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() >= 4 {
                // parts[3] should contain "checksmix 0.2.23"
                let name_and_version = parts[3];
                if let Some(version_part) = name_and_version.split_whitespace().nth(1) {
                    return Some(version_part.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_th_version() {
        let test_line = r#".TH MMIXASM 1 "May 2025" "checksmix 0.2.23""#;
        assert_eq!(extract_th_version(test_line), Some("0.2.23".to_string()));

        let test_line2 = r#".TH CHECKSMIX 1 "May 2025" "checksmix 0.2.23""#;
        assert_eq!(extract_th_version(test_line2), Some("0.2.23".to_string()));
    }
}
