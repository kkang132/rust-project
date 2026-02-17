mod analysis;
mod config;
mod pr;
mod report;

use clap::Parser;
use std::path::PathBuf;
use tracing::{debug, info, info_span};
use tracing_subscriber::EnvFilter;

/// PR Analyzer — CLI tool that takes a GitHub Pull Request URL and returns
/// a structured risk assessment across security, complexity, and style dimensions.
#[derive(Parser, Debug)]
#[command(name = "pr-analyzer", version, about)]
struct Cli {
    /// GitHub Pull Request URL (e.g., https://github.com/org/repo/pull/42)
    ///
    /// Not required when --mock is used.
    pr_url: Option<String>,

    /// Optional output file path for markdown report
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Use a built-in mock PR for demo purposes (no GitHub token needed)
    #[arg(long)]
    r#mock: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let pull_request = if cli.r#mock {
        info!("using mock PR data for demo");
        build_mock_pr()?
    } else {
        let pr_url = cli.pr_url.as_deref().ok_or(
            "PR URL is required unless --mock is used. Usage: pr-analyzer <URL> or pr-analyzer --mock",
        )?;

        let _main_span = info_span!("pr_analyze", pr_url = %pr_url).entered();

        info!("parsing PR URL");
        let parsed_url = pr::parse_pr_url(pr_url)?;
        debug!(owner = %parsed_url.owner, repo = %parsed_url.repo, pr = parsed_url.pr_number, "parsed PR URL");

        info!("loading configuration");
        let config = config::Config::load()?;

        info!("fetching pull request from GitHub");
        let fetched = pr::fetch_pull_request(&parsed_url, &config).await?;
        info!(files = fetched.files_changed, additions = fetched.additions, deletions = fetched.deletions, "fetched PR metadata");
        fetched
    };

    info!("running analysis");
    let results = analysis::run_all(&pull_request).await?;
    info!(analyzers = results.len(), "analysis complete");

    info!("generating report");
    let built_report = report::build(results, &pull_request);
    report::output(&built_report, cli.output.as_deref())?;
    info!(overall_risk = %built_report.overall_risk, "done");

    Ok(())
}

/// Build a mock PullRequest from the embedded sample diff fixture.
/// This enables running the full analysis pipeline without a GitHub token.
fn build_mock_pr() -> Result<pr::PullRequest, Box<dyn std::error::Error>> {
    let diff_text = include_str!("../tests/fixtures/sample_diff.patch");
    let files = pr::diff::parse_diff(diff_text)?;
    let additions: usize = files.iter().map(|f| f.additions).sum();
    let deletions: usize = files.iter().map(|f| f.deletions).sum();

    Ok(pr::PullRequest {
        number: 42,
        title: "Add OAuth2 login flow".to_string(),
        author: "alice".to_string(),
        files_changed: files.len(),
        additions,
        deletions,
        files,
    })
}
