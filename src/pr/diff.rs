use super::types::{DiffFile, Hunk};
use super::PrError;

/// Parse a unified diff string into a vector of DiffFile structs.
///
/// Codex: Implement unified diff parsing.
/// The input is the raw text from GitHub's diff endpoint.
///
/// Each file section starts with:
///   diff --git a/{path} b/{path}
///
/// New files have: `--- /dev/null`
/// Deleted files have: `+++ /dev/null`
///
/// Hunks start with: @@ -{old_start},{old_count} +{new_start},{new_count} @@
///
/// Lines are prefixed with:
///   '+' for additions
///   '-' for deletions
///   ' ' for context (unchanged)
pub fn parse_diff(_raw_diff: &str) -> Result<Vec<DiffFile>, PrError> {
    let raw_diff = _raw_diff;
    if raw_diff.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let mut current_file: Option<DiffFile> = None;
    let mut current_hunk: Option<Hunk> = None;

    let finish_hunk = |file: &mut Option<DiffFile>, hunk: &mut Option<Hunk>| {
        if let (Some(file), Some(hunk)) = (file.as_mut(), hunk.take()) {
            file.hunks.push(hunk);
        }
    };

    let finish_file =
        |files: &mut Vec<DiffFile>, file: &mut Option<DiffFile>, hunk: &mut Option<Hunk>| {
            finish_hunk(file, hunk);
            if let Some(file) = file.take() {
                files.push(file);
            }
        };

    for line in raw_diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            finish_file(&mut files, &mut current_file, &mut current_hunk);
            let mut parts = rest.split_whitespace();
            let a_path = parts
                .next()
                .ok_or_else(|| PrError::DiffParse("Missing a/ path in diff header".to_string()))?;
            let b_path = parts
                .next()
                .ok_or_else(|| PrError::DiffParse("Missing b/ path in diff header".to_string()))?;
            let path = b_path
                .strip_prefix("b/")
                .or_else(|| a_path.strip_prefix("a/"))
                .unwrap_or(b_path)
                .to_string();
            current_file = Some(DiffFile {
                path,
                is_new: false,
                is_deleted: false,
                additions: 0,
                deletions: 0,
                hunks: Vec::new(),
            });
            continue;
        }

        if line.starts_with("@@") {
            finish_hunk(&mut current_file, &mut current_hunk);
            let (old_start, old_count, new_start, new_count) = parse_hunk_header(line)?;
            current_hunk = Some(Hunk {
                old_start,
                old_count,
                new_start,
                new_count,
                lines: Vec::new(),
            });
            continue;
        }

        if line.starts_with("--- ") || line.starts_with("+++ ") {
            if let Some(file) = current_file.as_mut() {
                let path = line[4..].trim();
                if line.starts_with("--- ") && path == "/dev/null" {
                    file.is_new = true;
                }
                if line.starts_with("+++ ") && path == "/dev/null" {
                    file.is_deleted = true;
                }
            }
            continue;
        }

        if let (Some(file), Some(hunk)) = (current_file.as_mut(), current_hunk.as_mut()) {
            if line.starts_with('+') || line.starts_with('-') || line.starts_with(' ') {
                hunk.lines.push(line.to_string());
                if line.starts_with('+') && !line.starts_with("+++") {
                    file.additions += 1;
                } else if line.starts_with('-') && !line.starts_with("---") {
                    file.deletions += 1;
                }
            }
        }
    }

    finish_file(&mut files, &mut current_file, &mut current_hunk);
    Ok(files)
}

fn parse_hunk_header(line: &str) -> Result<(usize, usize, usize, usize), PrError> {
    let header = line
        .trim()
        .strip_prefix("@@")
        .ok_or_else(|| PrError::DiffParse("Invalid hunk header".to_string()))?
        .trim();
    let header = header.trim_end_matches("@@").trim();
    let mut parts = header.split_whitespace();
    let old_part = parts
        .next()
        .ok_or_else(|| PrError::DiffParse("Missing old range".to_string()))?;
    let new_part = parts
        .next()
        .ok_or_else(|| PrError::DiffParse("Missing new range".to_string()))?;

    let (old_start, old_count) = parse_range(old_part, '-')?;
    let (new_start, new_count) = parse_range(new_part, '+')?;

    Ok((old_start, old_count, new_start, new_count))
}

fn parse_range(part: &str, prefix: char) -> Result<(usize, usize), PrError> {
    let range = part
        .strip_prefix(prefix)
        .ok_or_else(|| PrError::DiffParse("Invalid range prefix".to_string()))?;
    let (start_str, count_str) = match range.split_once(',') {
        Some((start, count)) => (start, count),
        None => (range, "1"),
    };
    let start = start_str.parse::<usize>().map_err(|_| {
        PrError::DiffParse(format!("Invalid range start in {}", part))
    })?;
    let count = count_str.parse::<usize>().map_err(|_| {
        PrError::DiffParse(format!("Invalid range count in {}", part))
    })?;
    Ok((start, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sample unified diff for testing
    const SAMPLE_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,7 @@
 fn main() {
-    println!("old");
+    println!("new");
+    // Added a comment
 }
"#;

    #[test]
    fn test_parse_single_file_diff() {
        let files = parse_diff(SAMPLE_DIFF).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].additions, 2);
        assert_eq!(files[0].deletions, 1);
    }

    #[test]
    fn test_parse_new_file_diff() {
        let diff = r#"diff --git a/new_file.txt b/new_file.txt
new file mode 100644
index 0000000..e69de29
--- /dev/null
+++ b/new_file.txt
@@ -0,0 +1,2 @@
+hello
+world
"#;
        let files = parse_diff(diff).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].is_new);
        assert!(!files[0].is_deleted);
    }

    #[test]
    fn test_parse_deleted_file_diff() {
        let diff = r#"diff --git a/old_file.txt b/old_file.txt
deleted file mode 100644
index e69de29..0000000
--- a/old_file.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-hello
-world
"#;
        let files = parse_diff(diff).unwrap();
        assert_eq!(files.len(), 1);
        assert!(!files[0].is_new);
        assert!(files[0].is_deleted);
    }

    #[test]
    fn test_parse_empty_diff() {
        let files = parse_diff("").unwrap();
        assert!(files.is_empty());
    }
}
