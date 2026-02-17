use async_trait::async_trait;

use super::{Analyzer, AnalysisError};
use crate::pr::PullRequest;
use crate::report::types::{AnalysisResult, Finding, RiskLevel};

/// Complexity Risk Analyzer
///
/// Evaluates PR complexity across several dimensions:
/// - Number of new dependencies added
/// - Lines added/removed ratio
/// - Number of files changed
/// - New public API surface (exported types, functions)
/// - Nesting depth increases
pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check how many new dependencies are being added.
    fn check_dependency_count(&self, pr: &PullRequest) -> Vec<Finding> {
        let manifest_files = ["Cargo.toml", "package.json", "requirements.txt", "go.mod"];
        let mut findings = Vec::new();
        for file in &pr.files {
            let is_manifest = manifest_files.iter().any(|m| file.path.ends_with(m));
            if !is_manifest {
                continue;
            }
            let mut dep_count = 0usize;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = line[1..].trim();
                    if content.is_empty() || content.starts_with('[') || content.starts_with('#') {
                        continue;
                    }
                    if content.contains('=') || content.contains(':') || content.contains('/') {
                        dep_count += 1;
                    }
                }
            }
            if dep_count >= 3 {
                let severity = if dep_count >= 5 { RiskLevel::High } else { RiskLevel::Medium };
                findings.push(Finding {
                    message: format!("{} new dependencies added in {}", dep_count, file.path),
                    file: Some(file.path.clone()),
                    line: None,
                    severity,
                });
            }
        }
        findings
    }

    /// Evaluate the change size (lines added/removed, files changed).
    fn check_change_size(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        let total_changed = pr.additions + pr.deletions;

        if total_changed > 500 {
            findings.push(Finding {
                message: format!("Very large change: {} lines modified (+{} -{})", total_changed, pr.additions, pr.deletions),
                file: None,
                line: None,
                severity: RiskLevel::High,
            });
        } else if total_changed > 200 {
            findings.push(Finding {
                message: format!("Large change: {} lines modified (+{} -{})", total_changed, pr.additions, pr.deletions),
                file: None,
                line: None,
                severity: RiskLevel::Medium,
            });
        }

        if pr.files_changed > 20 {
            findings.push(Finding {
                message: format!("Very high number of files changed: {}", pr.files_changed),
                file: None,
                line: None,
                severity: RiskLevel::High,
            });
        } else if pr.files_changed > 10 {
            findings.push(Finding {
                message: format!("High number of files changed: {}", pr.files_changed),
                file: None,
                line: None,
                severity: RiskLevel::Medium,
            });
        }

        findings
    }

    /// Detect new public API surface introduced.
    fn check_api_surface(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        let pub_patterns = ["pub fn ", "pub struct ", "pub enum ", "pub trait ", "pub type "];
        let mut total_pub = 0usize;

        for file in &pr.files {
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = line[1..].trim_start();
                    if pub_patterns.iter().any(|p| content.starts_with(p)) {
                        total_pub += 1;
                        findings.push(Finding {
                            message: format!("New public API: {}", content.trim()),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::Low,
                        });
                    }
                }
            }
        }

        if total_pub > 10 {
            findings.push(Finding {
                message: format!("{} new public API items introduced — consider if all need to be public", total_pub),
                file: None,
                line: None,
                severity: RiskLevel::Medium,
            });
        }

        findings
    }

    /// Detect increases in nesting depth (deeply nested code).
    fn check_nesting_depth(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        for file in &pr.files {
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = &line[1..];
                    // Count leading whitespace to estimate nesting
                    let leading_spaces = content.len() - content.trim_start().len();
                    // 4 spaces per level, >4 levels = deeply nested
                    let indent_level = leading_spaces / 4;
                    if indent_level > 4 && !content.trim().is_empty() {
                        findings.push(Finding {
                            message: format!("Deeply nested code (indent level {}): consider refactoring", indent_level),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::Medium,
                        });
                    }
                }
            }
        }
        findings
    }
}

#[async_trait]
impl Analyzer for ComplexityAnalyzer {
    fn name(&self) -> &str {
        "Complexity Assessment"
    }

    async fn analyze(&self, pr: &PullRequest) -> Result<AnalysisResult, AnalysisError> {
        let mut findings = Vec::new();
        findings.extend(self.check_dependency_count(pr));
        findings.extend(self.check_change_size(pr));
        findings.extend(self.check_api_surface(pr));
        findings.extend(self.check_nesting_depth(pr));

        let risk_level = if findings.iter().any(|f| f.severity == RiskLevel::High) {
            RiskLevel::High
        } else if findings.iter().any(|f| f.severity == RiskLevel::Medium) {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        Ok(AnalysisResult {
            analyzer_name: self.name().to_string(),
            risk_level,
            findings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::tests::{test_diff_file, test_pull_request};

    #[tokio::test]
    async fn test_empty_pr_returns_low_complexity() {
        let analyzer = ComplexityAnalyzer::new();
        let pr = test_pull_request();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert_eq!(result.risk_level, RiskLevel::Low);
    }

    #[tokio::test]
    async fn test_large_pr_flags_high_complexity() {
        let mut pr = test_pull_request();
        pr.additions = 600;
        pr.deletions = 50;
        let analyzer = ComplexityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert_eq!(result.risk_level, RiskLevel::High);
        assert!(result.findings.iter().any(|f| f.message.contains("Very large change")));
    }

    #[tokio::test]
    async fn test_medium_pr_flags_medium_complexity() {
        let mut pr = test_pull_request();
        pr.additions = 250;
        pr.deletions = 50;
        let analyzer = ComplexityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert_eq!(result.risk_level, RiskLevel::Medium);
    }

    #[tokio::test]
    async fn test_many_files_flags_complexity() {
        let mut pr = test_pull_request();
        pr.files_changed = 15;
        let analyzer = ComplexityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert_eq!(result.risk_level, RiskLevel::Medium);
    }

    #[tokio::test]
    async fn test_detects_new_public_api() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/api.rs",
            vec![
                "+pub fn create_user(name: &str) -> User {".to_string(),
                "+pub struct User {".to_string(),
                "+pub enum Role {".to_string(),
            ],
        )];
        let analyzer = ComplexityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert_eq!(result.findings.iter().filter(|f| f.message.contains("New public API")).count(), 3);
    }

    #[tokio::test]
    async fn test_detects_deep_nesting() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/logic.rs",
            vec![
                "+                        deeply_nested_call();".to_string(), // 24 spaces = 6 levels
            ],
        )];
        let analyzer = ComplexityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(result.findings.iter().any(|f| f.message.contains("Deeply nested")));
    }
}
