/// Risk level for an analysis finding or overall assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
        }
    }
}

/// A single finding from an analyzer.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Human-readable description of the finding
    pub message: String,
    /// File path where the finding was detected (if applicable)
    pub file: Option<String>,
    /// Line number in the file (if applicable)
    pub line: Option<usize>,
    /// Severity of this individual finding
    pub severity: RiskLevel,
}

/// Result from a single analyzer run.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Name of the analyzer that produced this result
    pub analyzer_name: String,
    /// Overall risk level for this analysis dimension
    pub risk_level: RiskLevel,
    /// Individual findings
    pub findings: Vec<Finding>,
}

/// Complete report combining all analyzer results.
#[derive(Debug)]
pub struct Report {
    /// PR number
    pub pr_number: u64,
    /// PR title
    pub pr_title: String,
    /// PR author
    pub author: String,
    /// Files changed count
    pub files_changed: usize,
    /// Lines added
    pub additions: usize,
    /// Lines deleted
    pub deletions: usize,
    /// Results from each analyzer
    pub results: Vec<AnalysisResult>,
    /// Overall risk level (highest across all analyzers)
    pub overall_risk: RiskLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
    }

    #[test]
    fn test_risk_level_display() {
        assert_eq!(RiskLevel::Low.to_string(), "LOW");
        assert_eq!(RiskLevel::Medium.to_string(), "MEDIUM");
        assert_eq!(RiskLevel::High.to_string(), "HIGH");
    }

    #[test]
    fn test_finding_creation() {
        let finding = Finding {
            message: "SQL injection detected".to_string(),
            file: Some("db/query.rs".to_string()),
            line: Some(42),
            severity: RiskLevel::High,
        };
        assert_eq!(finding.severity, RiskLevel::High);
        assert_eq!(finding.file.as_deref(), Some("db/query.rs"));
    }
}
