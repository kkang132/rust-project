pub mod types;

pub use types::{AnalysisResult, Report, RiskLevel};
#[cfg(test)]
pub use types::Finding;

use crate::pr::PullRequest;
use colored::Colorize;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, instrument};

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("Failed to write report file: {0}")]
    FileWrite(#[from] std::io::Error),
}

/// Build a Report from analyzer results and PR metadata.
///
/// Claude: Implement.
/// Merge the Vec<AnalysisResult> with PullRequest metadata into a Report struct.
/// Compute overall_risk as the max risk level across all results.
pub fn build(results: Vec<AnalysisResult>, pr: &PullRequest) -> Report {
    let overall_risk = results
        .iter()
        .map(|r| r.risk_level)
        .max()
        .unwrap_or(RiskLevel::Low);

    Report {
        pr_number: pr.number,
        pr_title: pr.title.clone(),
        author: pr.author.clone(),
        files_changed: pr.files_changed,
        additions: pr.additions,
        deletions: pr.deletions,
        results,
        overall_risk,
    }
}

/// Output the report to terminal (default) or to a markdown file.
///
/// Claude: Implement both formatters.
/// - If output_path is None, print to stdout using colored terminal output
/// - If output_path is Some, write markdown to the specified file
#[instrument(skip(report), fields(pr = report.pr_number, overall_risk = %report.overall_risk))]
pub fn output(report: &Report, output_path: Option<&Path>) -> Result<(), ReportError> {
    match output_path {
        None => {
            debug!("writing report to terminal");
            print_terminal_report(report);
            Ok(())
        }
        Some(path) => {
            debug!(path = %path.display(), "writing report to file");
            write_markdown_report(report, path)
        }
    }
}

/// Format and print the report to the terminal with colors.
///
/// Claude: Implement terminal formatting.
/// Match the output format shown in AGENT.md:
///
/// PR #42: "Add OAuth2 login flow"
/// Author: alice | Files changed: 7 | +320 -45
///
/// ═══ Security Risk Assessment ═══
/// Risk Level: HIGH
/// • Finding 1
/// • Finding 2
/// ...
///
/// ═══ Overall Risk: HIGH ═══
fn print_terminal_report(report: &Report) {
    println!();
    println!(
        "PR #{}: \"{}\"",
        report.pr_number, report.pr_title
    );
    println!(
        "Author: {} | Files changed: {} | +{} -{}",
        report.author, report.files_changed, report.additions, report.deletions
    );
    println!();

    for result in &report.results {
        println!("═══ {} ═══", result.analyzer_name);
        println!("Risk Level: {}", colorize_risk(result.risk_level));
        if result.findings.is_empty() {
            println!("  No findings.");
        } else {
            for finding in &result.findings {
                let location = match (&finding.file, finding.line) {
                    (Some(f), Some(l)) => format!(" ({}:{})", f, l),
                    (Some(f), None) => format!(" ({})", f),
                    _ => String::new(),
                };
                println!("  • {}{}", finding.message, location);
            }
        }
        println!();
    }

    println!("═══ Overall Risk: {} ═══", colorize_risk(report.overall_risk));
    println!();
}

/// Write the report as a markdown file.
///
/// Claude: Implement markdown formatting.
/// Similar structure to terminal but using markdown syntax:
/// # PR #42: "Add OAuth2 login flow"
/// **Author:** alice | **Files changed:** 7 | **+320 -45**
///
/// ## Security Risk Assessment
/// **Risk Level: HIGH**
/// - Finding 1
/// - Finding 2
fn write_markdown_report(report: &Report, path: &Path) -> Result<(), ReportError> {
    let mut md = String::new();
    md.push_str(&format!("# PR #{}: \"{}\"\n\n", report.pr_number, report.pr_title));
    md.push_str(&format!(
        "**Author:** {} | **Files changed:** {} | **+{} -{}**\n\n",
        report.author, report.files_changed, report.additions, report.deletions
    ));

    for result in &report.results {
        md.push_str(&format!("## {}\n\n", result.analyzer_name));
        md.push_str(&format!("**Risk Level: {}**\n\n", result.risk_level));
        if result.findings.is_empty() {
            md.push_str("No findings.\n\n");
        } else {
            for finding in &result.findings {
                let location = match (&finding.file, finding.line) {
                    (Some(f), Some(l)) => format!(" (`{}:{}`)", f, l),
                    (Some(f), None) => format!(" (`{}`)", f),
                    _ => String::new(),
                };
                md.push_str(&format!("- **[{}]** {}{}\n", finding.severity, finding.message, location));
            }
            md.push('\n');
        }
    }

    md.push_str(&format!("## Overall Risk: {}\n", report.overall_risk));

    std::fs::write(path, md)?;
    Ok(())
}

/// Helper to colorize a risk level string for terminal output.
fn colorize_risk(level: RiskLevel) -> colored::ColoredString {
    match level {
        RiskLevel::High => "HIGH".red().bold(),
        RiskLevel::Medium => "MEDIUM".yellow().bold(),
        RiskLevel::Low => "LOW".green().bold(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pr::types::PullRequest;

    fn sample_pr() -> PullRequest {
        PullRequest {
            number: 42,
            title: "Add OAuth2 login flow".to_string(),
            author: "alice".to_string(),
            files_changed: 7,
            additions: 320,
            deletions: 45,
            files: vec![],
        }
    }

    #[test]
    fn test_build_report_overall_risk() {
        let results = vec![
            AnalysisResult {
                analyzer_name: "Security".to_string(),
                risk_level: RiskLevel::High,
                findings: vec![],
            },
            AnalysisResult {
                analyzer_name: "Complexity".to_string(),
                risk_level: RiskLevel::Low,
                findings: vec![],
            },
        ];
        let report = build(results, &sample_pr());
        assert_eq!(report.overall_risk, RiskLevel::High);
    }

    #[test]
    fn test_build_report_no_results() {
        let report = build(vec![], &sample_pr());
        assert_eq!(report.overall_risk, RiskLevel::Low);
    }

    #[test]
    fn test_build_report_metadata() {
        let report = build(vec![], &sample_pr());
        assert_eq!(report.pr_number, 42);
        assert_eq!(report.author, "alice");
        assert_eq!(report.additions, 320);
    }

    #[test]
    fn test_write_markdown_report() {
        let results = vec![
            AnalysisResult {
                analyzer_name: "Security".to_string(),
                risk_level: RiskLevel::High,
                findings: vec![Finding {
                    message: "SQL injection detected".to_string(),
                    file: Some("db/query.rs".to_string()),
                    line: Some(42),
                    severity: RiskLevel::High,
                }],
            },
        ];
        let report = build(results, &sample_pr());

        let dir = std::env::temp_dir();
        let path = dir.join("test_report.md");
        write_markdown_report(&report, &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# PR #42"));
        assert!(content.contains("**Author:** alice"));
        assert!(content.contains("## Security"));
        assert!(content.contains("SQL injection detected"));
        assert!(content.contains("## Overall Risk: HIGH"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_terminal_report_does_not_panic() {
        let results = vec![
            AnalysisResult {
                analyzer_name: "Security".to_string(),
                risk_level: RiskLevel::Low,
                findings: vec![],
            },
        ];
        let report = build(results, &sample_pr());
        // Just ensure it doesn't panic
        print_terminal_report(&report);
    }

    #[test]
    fn test_output_to_file() {
        let report = build(vec![], &sample_pr());
        let dir = std::env::temp_dir();
        let path = dir.join("test_output.md");
        output(&report, Some(&path)).unwrap();
        assert!(path.exists());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_output_to_terminal() {
        let report = build(vec![], &sample_pr());
        // Should not panic
        output(&report, None).unwrap();
    }
}
