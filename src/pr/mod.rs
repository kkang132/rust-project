pub mod diff;
pub mod types;

pub use types::{PrUrl, PullRequest};

use thiserror::Error;
use tracing::{debug, instrument};

#[derive(Debug, Error)]
pub enum PrError {
    #[error("GitHub API request failed: {0}")]
    ApiRequest(#[from] reqwest::Error),

    #[error("Invalid PR URL: {0}")]
    InvalidUrl(String),

    #[error("Failed to parse diff: {0}")]
    DiffParse(String),

    #[error("GitHub token not found in environment")]
    MissingToken,
}

/// Parse a GitHub PR URL into its component parts.
///
/// Codex: Implement URL parsing.
/// Expected format: https://github.com/{owner}/{repo}/pull/{number}
/// Return PrError::InvalidUrl for malformed URLs.
pub fn parse_pr_url(_url: &str) -> Result<PrUrl, PrError> {
    let parsed = reqwest::Url::parse(_url)
        .map_err(|_| PrError::InvalidUrl(_url.to_string()))?;

    if parsed.host_str() != Some("github.com") {
        return Err(PrError::InvalidUrl(_url.to_string()));
    }

    let segments: Vec<_> = parsed
        .path_segments()
        .ok_or_else(|| PrError::InvalidUrl(_url.to_string()))?
        .filter(|segment| !segment.is_empty())
        .collect();

    if segments.len() != 4 || segments[2] != "pull" {
        return Err(PrError::InvalidUrl(_url.to_string()));
    }

    let pr_number = segments[3]
        .parse::<u64>()
        .map_err(|_| PrError::InvalidUrl(_url.to_string()))?;

    Ok(PrUrl {
        owner: segments[0].to_string(),
        repo: segments[1].to_string(),
        pr_number,
    })
}

/// Fetch a complete PullRequest (metadata + parsed diff) from the GitHub API.
///
/// Codex: Implement using reqwest.
/// 1. Read GITHUB_TOKEN from env (return PrError::MissingToken if absent)
/// 2. GET /repos/{owner}/{repo}/pulls/{number} for metadata (JSON)
/// 3. GET the same endpoint with Accept: application/vnd.github.diff for raw diff
/// 4. Parse the diff using diff::parse_diff()
/// 5. Merge metadata + parsed diff into a PullRequest struct
#[instrument(skip(_config), fields(owner = %_pr_url.owner, repo = %_pr_url.repo, pr = _pr_url.pr_number))]
pub async fn fetch_pull_request(
    _pr_url: &PrUrl,
    _config: &crate::config::Config,
) -> Result<PullRequest, PrError> {
    let token = _config.github_token().ok_or(PrError::MissingToken)?;
    let client = reqwest::Client::new();
    let base_url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}",
        _pr_url.owner, _pr_url.repo, _pr_url.pr_number
    );

    #[derive(serde::Deserialize)]
    struct User {
        login: String,
    }

    #[derive(serde::Deserialize)]
    struct PullResponse {
        number: u64,
        title: String,
        user: User,
        changed_files: usize,
        additions: usize,
        deletions: usize,
    }

    debug!("fetching PR metadata from GitHub API");
    let response = client
        .get(&base_url)
        .header("User-Agent", "pr-analyzer")
        .bearer_auth(&token)
        .send()
        .await?
        .error_for_status()?;

    let metadata = response.json::<PullResponse>().await?;
    debug!(title = %metadata.title, changed_files = metadata.changed_files, "received PR metadata");

    debug!("fetching PR diff from GitHub API");
    let diff_text = client
        .get(&base_url)
        .header("User-Agent", "pr-analyzer")
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github.diff")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    debug!(diff_bytes = diff_text.len(), "received PR diff");

    let files = diff::parse_diff(&diff_text)?;
    debug!(parsed_files = files.len(), "parsed diff");

    Ok(PullRequest {
        number: metadata.number,
        title: metadata.title,
        author: metadata.user.login,
        files_changed: metadata.changed_files,
        additions: metadata.additions,
        deletions: metadata.deletions,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_pr_url() {
        let url = parse_pr_url("https://github.com/org/repo/pull/42").unwrap();
        assert_eq!(url.owner, "org");
        assert_eq!(url.repo, "repo");
        assert_eq!(url.pr_number, 42);
    }

    #[test]
    fn test_parse_invalid_pr_url() {
        assert!(parse_pr_url("https://example.com").is_err());
        assert!(parse_pr_url("not-a-url").is_err());
        assert!(parse_pr_url("https://github.com/org/repo/pulls/42").is_err());
    }
}
