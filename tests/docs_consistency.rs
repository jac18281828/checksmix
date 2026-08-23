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

#[test]
fn every_opcode_appears_once_in_the_instruction_table() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let opcodes = declared_opcode_names(manifest_dir);
    assert!(
        !opcodes.is_empty(),
        "could not find any `Opcode` variants in src/mmixal.rs -- parsing may be broken"
    );

    let mnemonic_counts = instruction_table_mnemonic_counts(manifest_dir);

    let missing: Vec<&String> = opcodes
        .iter()
        .filter(|op| !mnemonic_counts.contains_key(*op))
        .collect();
    let duplicated: Vec<(&String, usize)> = opcodes
        .iter()
        .filter_map(|op| {
            let count = *mnemonic_counts.get(op).unwrap_or(&0);
            if count > 1 { Some((op, count)) } else { None }
        })
        .collect();

    assert!(
        missing.is_empty() && duplicated.is_empty(),
        "MMIX.md's instruction table Mnemonic column diverges from `Opcode`: \
         missing {missing:?}, duplicated {duplicated:?}"
    );
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

/// Every variant name declared in `pub enum Opcode` in `src/mmixal.rs`, hand-
/// parsed the same way `declared_bin_names` reads `Cargo.toml` -- `Opcode`
/// has no reflection, and text-parsing its one enum block is the route
/// available without adding a dependency or an `Opcode::ALL`.
fn declared_opcode_names(manifest_dir: &str) -> Vec<String> {
    let source = std::fs::read_to_string(format!("{manifest_dir}/src/mmixal.rs"))
        .expect("Could not read src/mmixal.rs");

    let start = source
        .find("pub enum Opcode {")
        .expect("could not find `pub enum Opcode` in src/mmixal.rs");
    let body = &source[start..];
    let end = body
        .find('}')
        .expect("`pub enum Opcode` has no closing brace");

    body[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("//") || !line.contains('=') {
                return None;
            }
            line.split('=').next().map(|name| name.trim().to_string())
        })
        .collect()
}

/// How many times each backticked Mnemonic-column entry appears in
/// `MMIX.md`'s `## Instruction table` section. Scoped to that section alone:
/// opcode names also appear backticked in prose and in the skeleton code
/// block, and a whole-file scan would report duplicates that are not
/// duplicates.
fn instruction_table_mnemonic_counts(
    manifest_dir: &str,
) -> std::collections::HashMap<String, usize> {
    let doc =
        std::fs::read_to_string(format!("{manifest_dir}/MMIX.md")).expect("Could not read MMIX.md");

    let mut in_section = false;
    let mut counts = std::collections::HashMap::new();
    for line in doc.lines() {
        if line.trim() == "## Instruction table" {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## ") {
            break;
        }
        if !in_section {
            continue;
        }
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("| `") {
            let Some(mnemonic) = rest.split('`').next() else {
                continue;
            };
            *counts.entry(mnemonic.to_string()).or_insert(0) += 1;
        }
    }
    counts
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
