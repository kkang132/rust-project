use serde::Deserialize;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Top-level configuration loaded from .pr-analyzer.toml.
///
/// Codex: Implement loading logic.
/// All fields are optional — the tool should work with zero config.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// GitHub-specific settings
    #[serde(default)]
    pub github: GitHubConfig,

    /// Security analyzer settings (read from TOML config, consumed by future configurable patterns)
    #[serde(default)]
    #[allow(dead_code)]
    pub security: SecurityConfig,

    /// Style analyzer settings (read from TOML config, consumed by future configurable layers)
    #[serde(default)]
    #[allow(dead_code)]
    pub style: StyleConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubConfig {
    /// GitHub API token. If None, falls back to GITHUB_TOKEN env var.
    pub token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SecurityConfig {
    /// Additional regex patterns to flag as security risks
    #[serde(default)]
    #[allow(dead_code)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StyleConfig {
    /// Directories that define architectural layers (e.g., ["api", "domain", "infra"])
    #[serde(default)]
    #[allow(dead_code)]
    pub layers: Vec<String>,
}

impl Config {
    /// Load configuration from .pr-analyzer.toml in the current directory.
    /// Returns default config if the file doesn't exist.
    ///
    /// Codex: Implement file loading.
    /// 1. Check if .pr-analyzer.toml exists in the current directory
    /// 2. If yes, read and parse it
    /// 3. If no, return Config::default()
    /// 4. Override github.token with GITHUB_TOKEN env var if set
    pub fn load() -> Result<Config, ConfigError> {
        let path = Path::new(".pr-analyzer.toml");
        let mut config = if path.exists() {
            Self::load_from(path)?
        } else {
            Config::default()
        };

        if config.github.token.is_none() {
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                config.github.token = Some(token);
            }
        }

        Ok(config)
    }

    /// Load from a specific path (useful for testing).
    ///
    /// Codex: Implement for testability.
    pub fn load_from(path: &Path) -> Result<Config, ConfigError> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Resolve the GitHub token: config file value takes precedence,
    /// falls back to GITHUB_TOKEN env var.
    ///
    /// Codex: Implement token resolution.
    pub fn github_token(&self) -> Option<String> {
        self.github
            .token
            .clone()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.github.token.is_none());
        assert!(config.security.patterns.is_empty());
        assert!(config.style.layers.is_empty());
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[security]
patterns = ["TODO.*security"]

[style]
layers = ["api", "domain", "infra"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.security.patterns.len(), 1);
        assert_eq!(config.style.layers.len(), 3);
    }
}
