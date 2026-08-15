/// Integration test for docs consistency
/// Verifies every man page's version matches Cargo.toml, and that every
/// binary the crate declares has a man page. Discovers pages by enumerating
/// `man/*.1` rather than naming files, so a new binary's missing page is
/// caught without anyone remembering to wire it up here.

#[test]
fn man_page_versions_match_cargo_toml() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let pkg_version = env!("CARGO_PKG_VERSION");
    let pages = discover_man_pages(manifest_dir);
    assert!(
        !pages.is_empty(),
        "man/ contains no .1 pages -- discovery may be broken"
    );

    for page in &pages {
        let content = std::fs::read_to_string(page)
            .unwrap_or_else(|e| panic!("could not read {}: {}", page.display(), e));
        let version = extract_th_version(&content)
            .unwrap_or_else(|| panic!("could not extract .TH version from {}", page.display()));
        assert_eq!(
            version,
            pkg_version,
            "{} version mismatch: expected {}, got {}",
            page.display(),
            pkg_version,
            version
        );
    }
}

#[test]
fn man_page_exists_for_every_declared_binary() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bin_names = declared_bin_names(manifest_dir);
    assert!(
        !bin_names.is_empty(),
        "Cargo.toml declares no [[bin]] targets"
    );

    for name in &bin_names {
        let man_path = format!("{manifest_dir}/man/{name}.1");
        assert!(
            std::path::Path::new(&man_path).exists(),
            "binary '{name}' has no man page at man/{name}.1"
        );
    }
}

/// Every `.1` file directly under `man/`, sorted for deterministic failure
/// messages.
fn discover_man_pages(manifest_dir: &str) -> Vec<std::path::PathBuf> {
    let man_dir = format!("{manifest_dir}/man");
    let mut pages: Vec<std::path::PathBuf> = std::fs::read_dir(&man_dir)
        .unwrap_or_else(|e| panic!("could not read {man_dir}: {e}"))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("1"))
        .collect();
    pages.sort();
    pages
}

/// The `name` of every `[[bin]]` target in `Cargo.toml`, in file order.
/// Cargo.toml is parsed by hand rather than pulling in a TOML crate: the
/// crate's manifest has no other dependency on one, and the format needed
/// here -- `[[bin]]` sections with a `name = "..."` key -- is stable and
/// simple.
fn declared_bin_names(manifest_dir: &str) -> Vec<String> {
    let cargo_toml = std::fs::read_to_string(format!("{manifest_dir}/Cargo.toml"))
        .expect("Could not read Cargo.toml");

    let mut names = Vec::new();
    let mut in_bin_section = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed == "[[bin]]" {
            in_bin_section = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_bin_section = false;
            continue;
        }
        if in_bin_section
            && let Some(rest) = trimmed.strip_prefix("name")
            && let Some(value) = rest.trim_start().strip_prefix('=')
        {
            names.push(value.trim().trim_matches('"').to_string());
        }
    }
    names
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
