//! Source preprocessing: whole-line comment blanking, `debug` directive expansion, and `INCLUDE` resolution.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]

use super::DebugDirectiveOverflow;
use super::MMixAssembler;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;

impl MMixAssembler {
    /// Blank out every line whose first character (column 1, before any
    /// leading blank) is not a letter, digit, `:` or `_` -- the MMIXAL
    /// reference's whole-line comment rule. Line-count-preserving, like
    /// `preprocess_debug`, and run before it so a comment line's text can
    /// never be mistaken for a `debug` directive. An indented line is not
    /// covered: its content parses normally, blank or not.
    pub(super) fn blank_whole_line_comments(source: &str) -> String {
        let mut result = String::with_capacity(source.len());
        for line in source.split_inclusive('\n') {
            let content = line.strip_suffix('\n').unwrap_or(line);
            let is_comment_line = match content.chars().next() {
                Some(' ') | Some('\t') | None => false,
                Some(c) => !(c.is_ascii_alphanumeric() || c == ':' || c == '_'),
            };
            if is_comment_line {
                if content.len() != line.len() {
                    result.push('\n');
                }
            } else {
                result.push_str(line);
            }
        }
        result
    }

    /// Rewrite each `debug "text"` directive in `source` into
    /// `TRAP 0,Debug,K` on the directive's own line and label, and collect
    /// its decoded text. `start_index` is the `K` the first directive here
    /// receives — the count of directives every earlier translation unit
    /// contributed. Nothing is written to guest memory and no label is
    /// generated, so `K` costs one byte and the directive costs one tetra.
    ///
    /// Returns the preprocessed source, the strings this source's
    /// directives contributed (in `K` order), and the file/line/column of
    /// the first directive to exceed the table's 256-entry limit, if any —
    /// `parse` turns that into an assembly error rather than emitting a `K`
    /// that cannot fit `TRAP`'s one-byte `Z`.
    pub(super) fn preprocess_debug(
        source: &str,
        filename: &str,
        start_index: usize,
    ) -> (String, Vec<Vec<u8>>, DebugDirectiveOverflow) {
        let mut result = String::new();
        let mut strings = Vec::new();
        let mut overflow = None;

        for (index, line) in source.lines().enumerate() {
            match match_debug_line(line) {
                Some((label, text)) => {
                    let k = start_index + strings.len();
                    if k > 255 {
                        if overflow.is_none() {
                            let col = label.map_or(0, |l| l.chars().count()) + 1;
                            overflow = Some((filename.to_string(), index + 1, col));
                        }
                    } else {
                        result.push_str(label.map(str::trim).unwrap_or(""));
                        result.push_str(&format!("\tTRAP\t0,Debug,{k}\n"));
                    }
                    // debug text keeps the source's UTF-8 bytes: it lives
                    // outside guest memory, going straight to the host's
                    // handle 1, not through a data directive's per-character
                    // value.
                    strings.push(text.as_bytes().to_vec());
                }
                None => {
                    result.push_str(line);
                    result.push('\n');
                }
            }
        }

        debug!("Preprocessed source:\n{}", result);
        (result, strings, overflow)
    }

    /// Expand `INCLUDE <file>` directives into an ordered list of translation
    /// units, ready to feed to `new`/`add_source`. Host files are split at
    /// each `INCLUDE` into segments (each tagged with the host's filename and
    /// blank-padded to keep absolute line numbers); each included file is
    /// resolved recursively and its units inserted at that position. Paths
    /// resolve relative to the including file's directory. `read` supplies
    /// file contents -- injected so this logic is testable without real
    /// filesystem access and reusable by any frontend. A cycle (re-entry on
    /// the current include chain) or an unreadable file is an `Err`.
    pub fn resolve_includes<R>(
        root_source: &str,
        root_filename: &str,
        base_dir: &Path,
        read: &R,
    ) -> Result<Vec<(String, String)>, String>
    where
        R: Fn(&Path) -> std::io::Result<String>,
    {
        let mut chain = Vec::new();
        Self::resolve_includes_chain(root_source, root_filename, base_dir, read, &mut chain)
    }

    /// Recursive worker behind `resolve_includes`. `chain` holds the
    /// lexically-normalized identity of every file currently being expanded
    /// (the ancestors on the path from the top-level call to here), used to
    /// detect a file re-entering itself before it finishes expanding.
    fn resolve_includes_chain<R>(
        root_source: &str,
        root_filename: &str,
        base_dir: &Path,
        read: &R,
        chain: &mut Vec<PathBuf>,
    ) -> Result<Vec<(String, String)>, String>
    where
        R: Fn(&Path) -> std::io::Result<String>,
    {
        let mut units = Vec::new();
        let mut segment = String::new();
        let mut segment_start_line: usize = 1;
        let mut current_line: usize = 0;

        for raw_line in root_source.split_inclusive('\n') {
            current_line += 1;
            let content = raw_line.strip_suffix('\n').unwrap_or(raw_line);
            let Some(operand) = Self::parse_include_operand(content) else {
                segment.push_str(raw_line);
                continue;
            };

            if !segment.trim().is_empty() {
                units.push((
                    root_filename.to_string(),
                    Self::pad_source(&segment, segment_start_line),
                ));
            }
            segment.clear();

            let target = Self::normalize_lexically(&base_dir.join(&operand));
            let include_col = content.chars().take_while(|c| c.is_whitespace()).count() + 1;
            if chain.contains(&target) {
                let mut names: Vec<String> =
                    chain.iter().map(|p| p.display().to_string()).collect();
                names.push(target.display().to_string());
                return Err(format!(
                    "{root_filename}:{current_line}:{include_col}: include cycle detected: {}",
                    names.join(" -> ")
                ));
            }

            let included_source = read(&target).map_err(|err| {
                format!(
                    "{root_filename}:{current_line}:{include_col}: cannot read included file '{}': {}",
                    target.display(),
                    err
                )
            })?;
            let included_filename = target.display().to_string();
            let included_base_dir = target
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(""));

            chain.push(target);
            let included_units = Self::resolve_includes_chain(
                &included_source,
                &included_filename,
                &included_base_dir,
                read,
                chain,
            )?;
            chain.pop();
            units.extend(included_units);

            segment_start_line = current_line + 1;
        }

        if !segment.trim().is_empty() {
            units.push((
                root_filename.to_string(),
                Self::pad_source(&segment, segment_start_line),
            ));
        }

        Ok(units)
    }

    /// If `line`, after stripping a trailing `%` comment and trimming
    /// whitespace, is an `INCLUDE` directive matched in upper case only,
    /// returns its operand (unquoted if wrapped in matching double quotes).
    /// `;` is not a comment character here: it lands inside the operand and
    /// fails as an unreadable file naming the whole text, since `INCLUDE`
    /// occupies its own line.
    fn parse_include_operand(line: &str) -> Option<String> {
        let without_comment = match line.find('%') {
            Some(idx) => &line[..idx],
            None => line,
        };
        let trimmed = without_comment.trim();
        let mut parts = trimmed.splitn(2, |c: char| c.is_whitespace());
        let keyword = parts.next()?;
        if keyword != "INCLUDE" {
            return None;
        }
        let operand = parts.next().unwrap_or("").trim();
        if operand.is_empty() {
            return None;
        }
        let unquoted = if operand.len() >= 2 && operand.starts_with('"') && operand.ends_with('"') {
            &operand[1..operand.len() - 1]
        } else {
            operand
        };
        Some(unquoted.to_string())
    }

    /// Prepend `start_line - 1` newlines to `text` so absolute line numbers
    /// survive being embedded at `start_line` of some larger file (the
    /// grammar ignores leading blank lines).
    fn pad_source(text: &str, start_line: usize) -> String {
        let padding = start_line.saturating_sub(1);
        let mut padded = String::with_capacity(text.len() + padding);
        for _ in 0..padding {
            padded.push('\n');
        }
        padded.push_str(text);
        padded
    }

    /// Lexically collapse `.`/`..` components without touching the
    /// filesystem (unlike `fs::canonicalize`, which would defeat the
    /// injected reader in unit tests and can fail on a nonexistent path).
    fn normalize_lexically(path: &Path) -> PathBuf {
        use std::path::Component;

        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if !result.pop() {
                        result.push("..");
                    }
                }
                other => result.push(other.as_os_str()),
            }
        }
        result
    }
}

/// Matches one line against a `debug "text"` directive: an optional
/// label (blanks included, for the caller to trim and to count toward
/// its overflow column) and the quoted text. A blank is any
/// `char::is_whitespace` character.
fn match_debug_line(line: &str) -> Option<(Option<&str>, &str)> {
    if let Some((label_end, text)) = match_labeled_debug(line) {
        return Some((Some(&line[..label_end]), text));
    }
    let text = match_debug_after(line, 0)?;
    Some((None, text))
}

/// The labeled form: a run of blanks, alone or after a leading token,
/// directly ahead of the keyword. Returns the label's byte length and
/// the captured text.
fn match_labeled_debug(line: &str) -> Option<(usize, &str)> {
    let ws_start = line.char_indices().find(|(_, c)| c.is_whitespace())?.0;
    let label_end = line[ws_start..]
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(line.len(), |(i, _)| ws_start + i);
    let text = match_debug_after(line, label_end)?;
    Some((label_end, text))
}

/// The keyword, its required trailing blank, and the quoted text,
/// starting at `start`. Rejects a missing blank or opening quote, an
/// unterminated text, and any non-blank text after the closing quote.
fn match_debug_after(line: &str, start: usize) -> Option<&str> {
    let rest = line.get(start..)?.strip_prefix("debug")?;
    let ws_end = rest
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(rest.len(), |(i, _)| i);
    if ws_end == 0 {
        return None;
    }
    let after_quote = rest[ws_end..].strip_prefix('"')?;
    let close = after_quote.find('"')?;
    if after_quote[close + 1..].chars().any(|c| !c.is_whitespace()) {
        return None;
    }
    Some(&after_quote[..close])
}
