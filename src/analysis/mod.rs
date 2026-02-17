pub mod complexity;
pub mod security;
pub mod style;

use async_trait::async_trait;
use thiserror::Error;
use tracing::{debug, info_span, Instrument};

use crate::pr::PullRequest;
use crate::report::types::AnalysisResult;

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Analysis failed for {analyzer}: {reason}")]
    #[allow(dead_code)] // Used by future analyzer implementations
    Failed {
        analyzer: String,
        reason: String,
    },
}

/// Core trait that all risk analyzers must implement.
/// Analyzers must be Send + Sync to run concurrently via tokio::join!.
#[async_trait]
pub trait Analyzer: Send + Sync {
    /// Human-readable name of this analyzer (e.g., "Security Risk Assessment")
    fn name(&self) -> &str;

    /// Run the analysis on the given pull request and return structured results.
    /// Must not print to stdout — return findings via AnalysisResult.
    async fn analyze(&self, pr: &PullRequest) -> Result<AnalysisResult, AnalysisError>;
}

/// Run all three analyzers concurrently and collect their results.
///
/// Claude: Implement using tokio::join! to run SecurityAnalyzer,
/// ComplexityAnalyzer, and StyleAnalyzer in parallel.
///
/// Returns a Vec<AnalysisResult> with one entry per analyzer,
/// or propagates the first error encountered.
pub async fn run_all(pr: &PullRequest) -> Result<Vec<AnalysisResult>, AnalysisError> {
    let security = security::SecurityAnalyzer::new();
    let complexity = complexity::ComplexityAnalyzer::new();
    let style = style::StyleAnalyzer::new();

    let (sec_result, comp_result, style_result) = tokio::join!(
        security.analyze(pr).instrument(info_span!("analyze", analyzer = "security")),
        complexity.analyze(pr).instrument(info_span!("analyze", analyzer = "complexity")),
        style.analyze(pr).instrument(info_span!("analyze", analyzer = "style")),
    );

    let results = vec![sec_result?, comp_result?, style_result?];
    for r in &results {
        debug!(analyzer = %r.analyzer_name, risk = %r.risk_level, findings = r.findings.len(), "analyzer result");
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pr::types::{DiffFile, PullRequest};

    /// Helper to create a minimal PullRequest for testing.
    pub fn test_pull_request() -> PullRequest {
        PullRequest {
            number: 1,
            title: "Test PR".to_string(),
            author: "testuser".to_string(),
            files_changed: 0,
            additions: 0,
            deletions: 0,
            files: vec![],
        }
    }

    /// Helper to create a DiffFile with custom content for testing.
    pub fn test_diff_file(path: &str, lines: Vec<String>) -> DiffFile {
        use crate::pr::types::Hunk;
        DiffFile {
            path: path.to_string(),
            is_new: false,
            is_deleted: false,
            additions: lines.iter().filter(|l| l.starts_with('+')).count(),
            deletions: lines.iter().filter(|l| l.starts_with('-')).count(),
            hunks: vec![Hunk {
                old_start: 1,
                old_count: 10,
                new_start: 1,
                new_count: 10,
                lines,
            }],
        }
    }

    #[tokio::test]
    async fn test_run_all_returns_three_results() {
        let pr = test_pull_request();
        let results = run_all(&pr).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_run_all_analyzer_names() {
        let pr = test_pull_request();
        let results = run_all(&pr).await.unwrap();
        let names: Vec<&str> = results.iter().map(|r| r.analyzer_name.as_str()).collect();
        assert!(names.contains(&"Security Risk Assessment"));
        assert!(names.contains(&"Complexity Assessment"));
        assert!(names.contains(&"Style & Architecture Assessment"));
    }

    #[tokio::test]
    async fn test_run_all_with_dirty_pr() {
        let mut pr = test_pull_request();
        pr.additions = 600;
        pr.files = vec![test_diff_file(
            "src/main.rs",
            vec![
                "+    let password = \"secret123\";".to_string(),
                "+    unsafe {".to_string(),
                "+        todo!(\"fix this\")".to_string(),
            ],
        )];
        let results = run_all(&pr).await.unwrap();
        assert_eq!(results.len(), 3);
        // At least one analyzer should flag something
        assert!(results.iter().any(|r| !r.findings.is_empty()));
    }
}
