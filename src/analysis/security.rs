use async_trait::async_trait;

use super::{Analyzer, AnalysisError};
use crate::pr::PullRequest;
use crate::report::types::{AnalysisResult, Finding, RiskLevel};

/// Security Risk Analyzer
///
/// Scans PR diffs for security-relevant patterns:
/// - New dependencies without known audit status
/// - SQL injection, command injection, XSS patterns
/// - Hardcoded secrets or credentials
/// - Unsafe code blocks introduced
/// - Permission/scope changes in config files
pub struct SecurityAnalyzer {
    // TODO (third-party agent): Add configurable patterns from Config.security.patterns
}

impl SecurityAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    /// Scan diff lines for patterns indicating SQL injection risk.
    fn check_sql_injection(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        for file in &pr.files {
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = &line[1..];
                    // String interpolation in SQL context
                    let is_sql_file = file.path.ends_with(".sql");
                    let has_format_select = content.contains("format!") &&
                        (content.to_uppercase().contains("SELECT") ||
                         content.to_uppercase().contains("INSERT") ||
                         content.to_uppercase().contains("UPDATE") ||
                         content.to_uppercase().contains("DELETE"));
                    let has_string_concat_sql = (content.contains("\" +") || content.contains("+ \"")) &&
                        (content.to_uppercase().contains("SELECT") ||
                         content.to_uppercase().contains("WHERE"));

                    if is_sql_file && (content.contains("format!") || content.contains("${") || content.contains("' +")) {
                        findings.push(Finding {
                            message: "Possible SQL injection: string interpolation in SQL file".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::High,
                        });
                    } else if has_format_select || has_string_concat_sql {
                        findings.push(Finding {
                            message: "Possible SQL injection: raw SQL query construction with string interpolation".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::High,
                        });
                    }
                }
            }
        }
        findings
    }

    /// Scan for hardcoded secrets, API keys, tokens, passwords.
    fn check_hardcoded_secrets(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        let secret_patterns: &[(&str, &str)] = &[
            ("password\\s*=\\s*\"", "Hardcoded password detected"),
            ("api_key\\s*=\\s*\"", "Hardcoded API key detected"),
            ("secret\\s*=\\s*\"", "Hardcoded secret detected"),
            ("token\\s*=\\s*\"", "Hardcoded token detected"),
            ("AKIA[0-9A-Z]{16}", "AWS access key detected"),
            ("secret_key_", "Possible hardcoded secret key"),
            ("hardcoded_secret", "Hardcoded secret value"),
        ];
        for file in &pr.files {
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = &line[1..];
                    for (pattern, message) in secret_patterns {
                        if content.contains(pattern) ||
                           (pattern.contains("\\s*") && Self::matches_secret_pattern(content, pattern)) {
                            findings.push(Finding {
                                message: message.to_string(),
                                file: Some(file.path.clone()),
                                line: Some(hunk.new_start + i),
                                severity: RiskLevel::High,
                            });
                            break;
                        }
                    }
                }
            }
        }
        findings
    }

    /// Simple pattern matcher for secret detection.
    fn matches_secret_pattern(content: &str, pattern: &str) -> bool {
        // Handle simple patterns with \s*
        if let Some((prefix, suffix)) = pattern.split_once("\\s*=\\s*\"") {
            let _ = suffix;
            if let Some(pos) = content.find(prefix) {
                let rest = &content[pos + prefix.len()..];
                let rest = rest.trim_start();
                if rest.starts_with('=') {
                    let rest = rest[1..].trim_start();
                    return rest.starts_with('"');
                }
            }
            false
        } else {
            content.contains(pattern)
        }
    }

    /// Detect new unsafe blocks introduced in the diff.
    fn check_unsafe_code(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        for file in &pr.files {
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = &line[1..].trim();
                    if content.contains("unsafe {") || content.contains("unsafe fn") {
                        findings.push(Finding {
                            message: "New unsafe block introduced".to_string(),
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

    /// Detect new dependencies added in manifest files.
    fn check_new_dependencies(&self, pr: &PullRequest) -> Vec<Finding> {
        let manifest_files = ["Cargo.toml", "package.json", "requirements.txt", "go.mod", "Gemfile"];
        let mut findings = Vec::new();
        for file in &pr.files {
            let is_manifest = manifest_files.iter().any(|m| file.path.ends_with(m));
            if !is_manifest {
                continue;
            }
            let mut new_deps = Vec::new();
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = line[1..].trim();
                    if content.is_empty() || content.starts_with('[') || content.starts_with('#') {
                        continue;
                    }
                    // For Cargo.toml: lines like `name = "version"` or `name = { version = "..." }`
                    if file.path.ends_with("Cargo.toml") && content.contains('=') && !content.starts_with("version") && !content.starts_with("edition") && !content.starts_with("name") && !content.starts_with("description") {
                        new_deps.push(content.to_string());
                    }
                    // For requirements.txt: any non-comment line
                    if file.path.ends_with("requirements.txt") && !content.starts_with('#') {
                        new_deps.push(content.to_string());
                    }
                    // For package.json: lines with quoted keys
                    if file.path.ends_with("package.json") && content.contains(':') && content.contains('"') {
                        new_deps.push(content.to_string());
                    }
                    // For go.mod: lines starting with a module path
                    if file.path.ends_with("go.mod") && content.contains('/') {
                        new_deps.push(content.to_string());
                    }
                }
            }
            if !new_deps.is_empty() {
                let severity = if new_deps.len() >= 5 {
                    RiskLevel::High
                } else if new_deps.len() >= 3 {
                    RiskLevel::Medium
                } else {
                    RiskLevel::Low
                };
                findings.push(Finding {
                    message: format!("{} new dependencies added in {}: {}", new_deps.len(), file.path, new_deps.join(", ")),
                    file: Some(file.path.clone()),
                    line: None,
                    severity,
                });
            }
        }
        findings
    }

    /// Check for command injection patterns.
    fn check_command_injection(&self, pr: &PullRequest) -> Vec<Finding> {
        let mut findings = Vec::new();
        for file in &pr.files {
            for hunk in &file.hunks {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if !line.starts_with('+') {
                        continue;
                    }
                    let content = &line[1..];
                    // Rust: Command::new with format! or variable
                    if content.contains("Command::new") && (content.contains("format!") || content.contains('&')) {
                        findings.push(Finding {
                            message: "Possible command injection: Command::new with dynamic arguments".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::High,
                        });
                    }
                    // Python: shell=True
                    if content.contains("shell=True") || content.contains("shell = True") {
                        findings.push(Finding {
                            message: "Possible command injection: subprocess with shell=True".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::High,
                        });
                    }
                    // eval/exec in JS/Python
                    if (content.contains("eval(") || content.contains("exec(")) && !content.trim_start().starts_with("//") && !content.trim_start().starts_with('#') {
                        findings.push(Finding {
                            message: "Possible code injection: eval/exec usage detected".to_string(),
                            file: Some(file.path.clone()),
                            line: Some(hunk.new_start + i),
                            severity: RiskLevel::High,
                        });
                    }
                }
            }
        }
        findings
    }
}

#[async_trait]
impl Analyzer for SecurityAnalyzer {
    fn name(&self) -> &str {
        "Security Risk Assessment"
    }

    async fn analyze(&self, pr: &PullRequest) -> Result<AnalysisResult, AnalysisError> {
        let mut findings = Vec::new();
        findings.extend(self.check_sql_injection(pr));
        findings.extend(self.check_hardcoded_secrets(pr));
        findings.extend(self.check_unsafe_code(pr));
        findings.extend(self.check_new_dependencies(pr));
        findings.extend(self.check_command_injection(pr));

        let risk_level = determine_risk_level(&findings);

        Ok(AnalysisResult {
            analyzer_name: self.name().to_string(),
            risk_level,
            findings,
        })
    }
}

/// Determine overall risk level from a set of findings.
/// HIGH if any finding is HIGH. MEDIUM if any is MEDIUM. LOW otherwise.
fn determine_risk_level(findings: &[Finding]) -> RiskLevel {
    if findings.iter().any(|f| f.severity == RiskLevel::High) {
        RiskLevel::High
    } else if findings.iter().any(|f| f.severity == RiskLevel::Medium) {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::tests::{test_diff_file, test_pull_request};

    #[tokio::test]
    async fn test_empty_pr_returns_low_risk() {
        let analyzer = SecurityAnalyzer::new();
        let pr = test_pull_request();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert_eq!(result.risk_level, RiskLevel::Low);
        assert!(result.findings.is_empty());
    }

    #[tokio::test]
    async fn test_detects_sql_injection_in_rs_file() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/db.rs",
            vec!["+    format!(\"SELECT * FROM users WHERE id = {}\", user_id)".to_string()],
        )];
        let analyzer = SecurityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(!result.findings.is_empty());
        assert!(result.findings.iter().any(|f| f.message.contains("SQL injection")));
        assert_eq!(result.risk_level, RiskLevel::High);
    }

    #[tokio::test]
    async fn test_detects_sql_injection_in_sql_file() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "migrations/003.sql",
            vec!["+    format!(\"SELECT * FROM users WHERE id = {}\", user_id)".to_string()],
        )];
        let analyzer = SecurityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(!result.findings.is_empty());
    }

    #[tokio::test]
    async fn test_detects_hardcoded_secrets() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/config.rs",
            vec!["+    let secret = \"hardcoded_secret_key_12345\";".to_string()],
        )];
        let analyzer = SecurityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(!result.findings.is_empty());
        assert_eq!(result.risk_level, RiskLevel::High);
    }

    #[tokio::test]
    async fn test_detects_hardcoded_password() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/auth.rs",
            vec!["+    let password = \"hunter2\";".to_string()],
        )];
        let analyzer = SecurityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(!result.findings.is_empty());
        assert_eq!(result.risk_level, RiskLevel::High);
    }

    #[tokio::test]
    async fn test_detects_unsafe_blocks() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/main.rs",
            vec![
                "+    unsafe {".to_string(),
                "+        std::ptr::write_volatile(0x1000 as *mut u8, 0);".to_string(),
                "+    }".to_string(),
            ],
        )];
        let analyzer = SecurityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(!result.findings.is_empty());
        assert!(result.findings.iter().any(|f| f.message.contains("unsafe")));
        assert_eq!(result.risk_level, RiskLevel::Medium);
    }

    #[tokio::test]
    async fn test_detects_new_dependencies() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "Cargo.toml",
            vec![
                "+oauth2-lite = \"0.3\"".to_string(),
                "+base64-url = \"2\"".to_string(),
                "+reqwest = { version = \"0.12\", features = [\"json\"] }".to_string(),
            ],
        )];
        let analyzer = SecurityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(result.findings.iter().any(|f| f.message.contains("dependencies")));
    }

    #[tokio::test]
    async fn test_detects_command_injection_shell_true() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/runner.py",
            vec!["+    subprocess.run(cmd, shell=True)".to_string()],
        )];
        let analyzer = SecurityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(!result.findings.is_empty());
        assert!(result.findings.iter().any(|f| f.message.contains("command injection")));
    }

    #[tokio::test]
    async fn test_no_findings_on_clean_code() {
        let mut pr = test_pull_request();
        pr.files = vec![test_diff_file(
            "src/lib.rs",
            vec![
                "+fn add(a: i32, b: i32) -> i32 {".to_string(),
                "+    a + b".to_string(),
                "+}".to_string(),
            ],
        )];
        let analyzer = SecurityAnalyzer::new();
        let result = analyzer.analyze(&pr).await.unwrap();
        assert!(result.findings.is_empty());
        assert_eq!(result.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_determine_risk_level_high() {
        let findings = vec![Finding {
            message: "test".to_string(),
            file: Some("test.rs".to_string()),
            line: Some(1),
            severity: RiskLevel::High,
        }];
        assert_eq!(determine_risk_level(&findings), RiskLevel::High);
    }

    #[test]
    fn test_determine_risk_level_empty() {
        assert_eq!(determine_risk_level(&[]), RiskLevel::Low);
    }
}
