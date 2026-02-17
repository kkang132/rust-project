/// Metadata about a pull request fetched from the GitHub API.
/// Codex: Populate all fields from the GitHub REST API response.
/// Note: Not Deserialize — PullRequest is constructed manually from
/// the GitHub API JSON response + parsed diff (DiffFile/Hunk).
#[derive(Debug, Clone)]
pub struct PullRequest {
    /// PR number (e.g., 42)
    pub number: u64,
    /// PR title
    pub title: String,
    /// Author's GitHub login
    pub author: String,
    /// Total files changed
    pub files_changed: usize,
    /// Total lines added
    pub additions: usize,
    /// Total lines deleted
    pub deletions: usize,
    /// Parsed diff files
    pub files: Vec<DiffFile>,
}

/// A single file within the PR diff.
/// Codex: Populated by the diff parser in diff.rs.
#[derive(Debug, Clone)]
pub struct DiffFile {
    /// File path (e.g., "src/auth/config.rs")
    pub path: String,
    /// Whether this is a new file
    pub is_new: bool,
    /// Whether this file was deleted
    pub is_deleted: bool,
    /// Lines added in this file
    pub additions: usize,
    /// Lines deleted in this file
    pub deletions: usize,
    /// Hunks (contiguous changed regions)
    pub hunks: Vec<Hunk>,
}

/// A contiguous region of changes within a file.
/// Codex: Parsed from unified diff format.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Structural diff fields populated by parser, consumed as API grows
pub struct Hunk {
    /// Starting line number in the old file
    pub old_start: usize,
    /// Number of lines in the old file
    pub old_count: usize,
    /// Starting line number in the new file
    pub new_start: usize,
    /// Number of lines in the new file
    pub new_count: usize,
    /// Raw lines of the hunk (prefixed with +, -, or space)
    pub lines: Vec<String>,
}

/// Represents the parsed components of a GitHub PR URL.
/// Codex: Extracted by parse_pr_url() in pr/mod.rs.
#[derive(Debug, Clone)]
pub struct PrUrl {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pr_url_fields() {
        let url = PrUrl {
            owner: "org".to_string(),
            repo: "repo".to_string(),
            pr_number: 42,
        };
        assert_eq!(url.owner, "org");
        assert_eq!(url.repo, "repo");
        assert_eq!(url.pr_number, 42);
    }
}
