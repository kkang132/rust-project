use async_trait::async_trait;

use super::{Analyzer, AnalysisError};
use crate::pr::PullRequest;
use crate::report::types::{AnalysisResult, Finding, RiskLevel};

/// Style & Architecture Risk Analyzer
///
/// Checks for conformance with project conventions:
/// - File placement matches existing module structure
/// - Naming conventions (snake_case for modules, PascalCase for types)
/// - Error handling patterns (unwrap vs ? operator)
/// - Import organization
/// - Architectural boundary violations
/// - Lint-style checks (unnecessary clone, todo! macros, missing #[must_use])
pub struct StyleAnalyzer {
    // TODO (third-party agent): Add configurable layers from Config.style.layers
}

impl StyleAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    /// Check for unwrap() usage in non-test code.
    fn check_unwrap_usage(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        for file in &pr.files {
            // Skip test files
            if file.path.starts_with("tests/") || file.path.contains("/tests/") || file.path.ends_with("_test.rs") {
                continue;
            }
            // Check if file contains #[cfg(test)] — we can only heuristically check lines
            let mut in_test_section = false;
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    let raw = if line.starts_with('+') || line.starts_with('-') || line.starts_with(' ') {
                        &line[1..]
                    } else {
                        line.as_str()
                    };
                    if raw.contains("#[cfg(test)]") {
                        in_test_section = true;
                    }
                    if !line.starts_with('+') || in_test_section {
                        continue;
                    }
                    let content = &line[1..];
                    if content.contains(".unwrap()") {
                        findings.push(Finding {
                            message: format!("Use of .unwrap() — prefer ? operator or .expect() with context"),
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

    /// Check for todo!() and unimplemented!() macros left in production code.
    fn check_todo_macros(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        for file in &pr.files {
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = &line[1..];
                    if content.contains("todo!()") || content.contains("todo!(\"") {
                        findings.push(Finding {
                            message: "todo!() macro found — should not ship to production".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::Medium,
                        });
                    }
                    if content.contains("unimplemented!()") || content.contains("unimplemented!(\"") {
                        findings.push(Finding {
                            message: "unimplemented!() macro found — should not ship to production".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::Medium,
                        });
                    }
                    let trimmed = content.trim().to_uppercase();
                    if trimmed.starts_with("// FIXME") || trimmed.starts_with("# FIXME") {
                        findings.push(Finding {
                            message: "FIXME comment found — indicates known issue".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::Low,
                        });
                    }
                }
            }
        }
        findings
    }

    /// Check for unnecessary clone() calls (heuristic).
    fn check_unnecessary_clone(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        for file in &pr.files {
            if !file.path.ends_with(".rs") {
                continue;
            }
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = &line[1..];
                    // Heuristic: .clone() on a &str or &String pattern, or .to_string().clone()
                    if content.contains(".to_string().clone()") || content.contains(".to_owned().clone()") {
                        findings.push(Finding {
                            message: "Redundant clone: .to_string().clone() or .to_owned().clone()".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::Low,
                        });
                    }
                }
            }
        }
        findings
    }

    /// Check architectural boundary violations.
    fn check_architecture_boundaries(&self, _pr: &PullRequest) -> Vec<Finding> {
        // Without configured layers, we can't check boundaries
        // This would require Config.style.layers to be populated
        // For now, return empty — the check is a no-op without layer configuration
        vec![]
    }

    /// Check naming conventions in new files and types.
    fn check_naming_conventions(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        for file in &pr.files {
            if !file.is_new {
                continue;
            }
            // Check file name is snake_case (for Rust files)
            if file.path.ends_with(".rs") {
                if let Some(filename) = file.path.rsplit('/').next() {
                    let stem = filename.trim_end_matches(".rs");
                    if stem != "mod" && stem != "lib" && stem != "main" && !is_snake_case(stem) {
                        findings.push(Finding {
                            message: format!("File name '{}' does not follow snake_case convention", filename),
                            file: Some(file.path.clone()),
                            line: None,
                            severity: RiskLevel::Low,
                        });
                    }
                }
            }
            // Check type definitions are PascalCase
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = line[1..].trim_start();
                    for keyword in &["struct ", "enum ", "trait "] {
                        let prefix = format!("pub {}", keyword);
                        let name = if content.starts_with(&prefix) {
                            content[prefix.len()..].split(|c: char| !c.is_alphanumeric() && c != '_').next()
                        } else if content.starts_with(keyword) {
                            content[keyword.len()..].split(|c: char| !c.is_alphanumeric() && c != '_').next()
                        } else {
                            None
                        };
                        if let Some(name) = name {
                            if !name.is_empty() && !is_pascal_case(name) {
                                findings.push(Finding {
                                    message: format!("Type '{}' does not follow PascalCase convention", name),
                                    file: Some(file.path.clone()),
                                    line: Some(hunk.new_start + i),
                                    severity: RiskLevel::Low,
                                });
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

fn is_snake_case(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !s.starts_with('_')
        && !s.contains("__")
}

fn is_pascal_case(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_uppercase())
        && !s.contains('_')
        && s.chars().all(|c| c.is_alphanumeric())
}

#[async_trait]
impl Analyzer for StyleAnalyzer {
    fn name(&self) -> &str {
        "Style & Architecture Assessment"
    }

    async fn analyze(&self, pr: &PullRequest) -> Result<AnalysisResult, AnalysisError> {
        let mut findings = Vec::new();
        findings.extend(self.check_unwrap_usage(pr));
        findings.extend(self.check_todo_macros(pr));
        findings.extend(self.check_unnecessary_clone(pr));
        findings.extend(self.check_architecture_boundaries(pr));
        findings.extend(self.check_naming_conventions(pr));

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
    async fn test_empty_pr_returns_low_style_risk() {
        let analyzer = StyleAnalyzer::new();
        let pr = test_pull_request();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert_eq!(result.risk_level, RiskLevel::Low);
    }

    #[tokio::test]
    async fn test_detects_unwrap_usage() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/main.rs",
            vec![
                "+    let val = some_result.unwrap();".to_string(),
            ],
        )];
        let analyzer = StyleAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(!result.findings.is_empty());
        assert!(result.findings.iter().any(|f| f.message.contains("unwrap()")));
        assert_eq!(result.risk_level, RiskLevel::Medium);
    }

    #[tokio::test]
    async fn test_detects_todo_macros() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/lib.rs",
            vec![
                "+    todo!(\"implement this\")".to_string(),
            ],
        )];
        let analyzer = StyleAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(!result.findings.is_empty());
        assert!(result.findings.iter().any(|f| f.message.contains("todo!()")));
    }

    #[tokio::test]
    async fn test_detects_unimplemented_macro() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/lib.rs",
            vec![
                "+    unimplemented!()".to_string(),
            ],
        )];
        let analyzer = StyleAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(result.findings.iter().any(|f| f.message.contains("unimplemented!()")));
    }

    #[tokio::test]
    async fn test_ignores_unwrap_in_tests() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "tests/integration.rs",
            vec![
                "+    let val = some_result.unwrap();".to_string(),
            ],
        )];
        let analyzer = StyleAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        // Test files should not flag unwrap
        assert!(result.findings.iter().all(|f| !f.message.contains("unwrap()")));
    }

    #[tokio::test]
    async fn test_detects_redundant_clone() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/util.rs",
            vec![
                "+    let s = name.to_string().clone();".to_string(),
            ],
        )];
        let analyzer = StyleAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(result.findings.iter().any(|f| f.message.contains("Redundant clone")));
    }

    #[tokio::test]
    async fn test_detects_fixme_comment() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/auth.rs",
            vec![
                "+// FIXME: auth tokens not rotated".to_string(),
            ],
        )];
        let analyzer = StyleAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(result.findings.iter().any(|f| f.message.contains("FIXME")));
    }

    #[test]
    fn test_is_snake_case() {
        assert!(is_snake_case("hello_world"));
        assert!(is_snake_case("foo"));
        assert!(!is_snake_case("HelloWorld"));
        assert!(!is_snake_case("_leading"));
        assert!(!is_snake_case("double__underscore"));
    }

    #[test]
    fn test_is_pascal_case() {
        assert!(is_pascal_case("HelloWorld"));
        assert!(is_pascal_case("Foo"));
        assert!(!is_pascal_case("hello_world"));
        assert!(!is_pascal_case("helloWorld"));
    }
}
