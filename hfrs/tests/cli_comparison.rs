mod helpers;

use helpers::{require_cli, require_token, CliRunner};

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
}

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
fn datasets_list_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs.run_json(&["datasets", "list", "--limit", "3"]).unwrap();
    let hf_out = hf.run_json(&["datasets", "list", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
}

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
}

#[test]
fn models_info_matches_hfjs() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hfjs = CliRunner::new("hfjs");
    require_cli(&hfjs);

    let hfrs_out = hfrs.run_json(&["models", "info", "gpt2"]).unwrap();
    let hfjs_out = hfjs.run_json(&["models", "info", "gpt2"]).unwrap();

    helpers::assert_json_equivalent(&hfrs_out, &hfjs_out, helpers::VOLATILE_FIELDS);
}
