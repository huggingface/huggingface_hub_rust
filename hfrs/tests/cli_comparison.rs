mod helpers;

use helpers::{require_cli, require_token, CliRunner};

// --- Basic smoke tests (no token needed) ---

#[test]
fn version_runs() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .arg("version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("hfrs "));
}

#[test]
fn env_runs() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .arg("env")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hfrs version:"));
    assert!(stdout.contains("Platform:"));
}

#[test]
fn help_shows_all_commands() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for cmd in &[
        "auth",
        "cache",
        "collections",
        "datasets",
        "discussions",
        "download",
        "endpoints",
        "jobs",
        "likes",
        "models",
        "papers",
        "repos",
        "spaces",
        "upload",
        "webhooks",
        "access-requests",
        "env",
        "version",
    ] {
        assert!(stdout.contains(cmd), "help output should contain command '{cmd}'");
    }
}

#[test]
fn models_help_shows_subcommands() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .args(["models", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("info"));
    assert!(stdout.contains("list"));
}

#[test]
fn repos_help_shows_subcommands() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .args(["repos", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for cmd in &["create", "delete", "move", "settings", "delete-files", "branch", "tag"] {
        assert!(stdout.contains(cmd), "repos help should contain subcommand '{cmd}'");
    }
}

#[test]
fn discussions_help_shows_subcommands() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .args(["discussions", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for cmd in &[
        "list", "info", "create", "comment", "merge", "close", "reopen", "rename", "diff",
    ] {
        assert!(stdout.contains(cmd), "discussions help should contain subcommand '{cmd}'");
    }
}

#[test]
fn jobs_help_shows_subcommands() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hfrs"))
        .args(["jobs", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for cmd in &[
        "run",
        "ps",
        "inspect",
        "cancel",
        "logs",
        "hardware",
        "stats",
        "scheduled",
    ] {
        assert!(stdout.contains(cmd), "jobs help should contain subcommand '{cmd}'");
    }
}

// --- Models comparison tests ---

#[test]
fn models_list_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs.run_json(&["models", "list", "--limit", "3"]).unwrap();
    let hf_out = hf.run_json(&["models", "list", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array(), "hfrs output should be an array");
    assert!(hf_out.is_array(), "hf output should be an array");
    assert_eq!(
        hfrs_out.as_array().unwrap().len(),
        hf_out.as_array().unwrap().len(),
        "Should return same number of models"
    );
}

#[test]
fn models_info_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs.run_json(&["models", "info", "gpt2"]).unwrap();
    let hf_out = hf.run_json(&["models", "info", "gpt2"]).unwrap();

    helpers::assert_json_equivalent(&hfrs_out, &hf_out, helpers::VOLATILE_FIELDS);
}

#[test]
fn models_list_with_search_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs.run_json(&["models", "list", "--search", "gpt2", "--limit", "3"]).unwrap();
    let hf_out = hf.run_json(&["models", "list", "--search", "gpt2", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
    assert_eq!(hfrs_out.as_array().unwrap().len(), hf_out.as_array().unwrap().len(),);
}

#[test]
fn models_list_with_author_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs
        .run_json(&["models", "list", "--author", "openai", "--limit", "3"])
        .unwrap();
    let hf_out = hf.run_json(&["models", "list", "--author", "openai", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
    assert_eq!(hfrs_out.as_array().unwrap().len(), hf_out.as_array().unwrap().len(),);
}

// --- Datasets comparison tests ---

#[test]
fn datasets_list_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs.run_json(&["datasets", "list", "--limit", "3"]).unwrap();
    let hf_out = hf.run_json(&["datasets", "list", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
    assert_eq!(hfrs_out.as_array().unwrap().len(), hf_out.as_array().unwrap().len(),);
}

#[test]
fn datasets_info_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs.run_json(&["datasets", "info", "squad"]).unwrap();
    let hf_out = hf.run_json(&["datasets", "info", "squad"]).unwrap();

    helpers::assert_json_equivalent(&hfrs_out, &hf_out, helpers::VOLATILE_FIELDS);
}

// --- Spaces comparison tests ---

#[test]
fn spaces_list_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs.run_json(&["spaces", "list", "--limit", "3"]).unwrap();
    let hf_out = hf.run_json(&["spaces", "list", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
    assert_eq!(hfrs_out.as_array().unwrap().len(), hf_out.as_array().unwrap().len(),);
}

// --- Auth comparison tests ---

#[test]
fn auth_whoami_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs.run_json(&["auth", "whoami"]).unwrap();
    let hf_out = hf.run_json(&["auth", "whoami"]).unwrap();

    // Both should have a username field
    assert!(
        hfrs_out.get("username").is_some() || hfrs_out.get("name").is_some(),
        "hfrs whoami should have username"
    );
    assert!(hf_out.get("username").is_some() || hf_out.get("name").is_some(), "hf whoami should have username");
}

// --- Papers comparison tests ---

#[test]
fn papers_list_returns_results() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let hfrs_out = hfrs.run_json(&["papers", "list", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array(), "papers list should return an array");
    assert!(!hfrs_out.as_array().unwrap().is_empty(), "papers list should return results");
}

#[test]
fn papers_search_returns_results() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let hfrs_out = hfrs.run_json(&["papers", "search", "transformer", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array(), "papers search should return an array");
}

// --- Collections comparison tests ---

#[test]
fn collections_list_returns_results() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let hfrs_out = hfrs.run_json(&["collections", "list", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array(), "collections list should return an array");
}

// --- Webhooks test ---

#[test]
fn webhooks_list_returns_array() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let hfrs_out = hfrs.run_json(&["webhooks", "list"]).unwrap();

    assert!(hfrs_out.is_array(), "webhooks list should return an array");
}
