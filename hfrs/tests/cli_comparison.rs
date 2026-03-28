mod helpers;

use helpers::{require_cli, require_token, require_write, CliRunner};

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
fn models_info_returns_valid_json() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["models", "info", "gpt2"]).unwrap();

    assert!(out.is_object(), "models info should return an object");
    let id = out.get("id").and_then(|v| v.as_str()).unwrap_or("");
    assert!(id == "gpt2" || id.ends_with("/gpt2"), "model id should be gpt2 or end with /gpt2, got: {id}");
    assert!(out.get("author").is_some());
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
fn datasets_info_returns_valid_json() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["datasets", "info", "squad"]).unwrap();

    assert!(out.is_object(), "datasets info should return an object");
    let id = out.get("id").and_then(|v| v.as_str()).unwrap_or("");
    assert!(id == "squad" || id.ends_with("/squad"), "dataset id should be squad or end with /squad, got: {id}");
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

// --- Auth tests ---

#[test]
fn auth_whoami_returns_valid_json() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["auth", "whoami"]).unwrap();

    assert!(out.is_object(), "whoami should return an object");
    assert!(out.get("username").is_some(), "whoami should have username field");
}

// --- Papers tests ---

#[test]
fn papers_list_returns_results() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["papers", "list", "--limit", "3"]).unwrap();

    assert!(out.is_array(), "papers list should return an array");
    assert!(!out.as_array().unwrap().is_empty(), "papers list should return results");
}

#[test]
fn papers_search_returns_results() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["papers", "search", "transformer", "--limit", "3"]).unwrap();

    assert!(out.is_array(), "papers search should return an array");
}

// --- Collections tests ---

#[test]
fn collections_list_returns_results() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["collections", "list", "--limit", "3"]).unwrap();

    assert!(out.is_array(), "collections list should return an array");
}

// --- Webhooks test ---

#[test]
fn webhooks_list_returns_array() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["webhooks", "list"]).unwrap();

    assert!(out.is_array(), "webhooks list should return an array");
}

// --- Table output tests ---

#[test]
fn models_list_table_output() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_raw(&["models", "list", "--limit", "3", "--format", "table"]).unwrap();

    assert!(out.contains("ID"), "table should have ID header");
    assert!(out.contains("Author"), "table should have Author header");
}

#[test]
fn models_list_quiet_output() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_raw(&["models", "list", "--limit", "3", "--quiet"]).unwrap();

    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines.len(), 3, "quiet mode should output 3 IDs");
    for line in &lines {
        assert!(!line.contains(' '), "quiet mode lines should be plain IDs, got: '{line}'");
    }
}

// --- Models field structure tests ---

#[test]
fn models_list_json_has_expected_fields() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["models", "list", "--limit", "5"]).unwrap();

    let items = out.as_array().expect("models list should return an array");
    assert!(!items.is_empty(), "models list should return results");
    for item in items {
        assert!(item.get("id").is_some(), "model item should have 'id' field");
        assert!(item.get("author").is_some(), "model item should have 'author' field");
        assert!(item.get("tags").is_some(), "model item should have 'tags' field");
    }
}

#[test]
fn models_list_sort_by_downloads() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs
        .run_json(&["models", "list", "--sort", "downloads", "--limit", "5"])
        .unwrap();

    let items = out.as_array().expect("models list should return an array");
    assert!(!items.is_empty(), "sorted models list should return results");
    let downloads: Vec<u64> = items
        .iter()
        .filter_map(|item| item.get("downloads").and_then(|d| d.as_u64()))
        .collect();
    for i in 1..downloads.len() {
        assert!(
            downloads[i - 1] >= downloads[i],
            "models should be sorted by downloads descending: {} < {} at index {}",
            downloads[i - 1],
            downloads[i],
            i
        );
    }
}

#[test]
fn models_info_gpt2_has_expected_structure() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["models", "info", "gpt2"]).unwrap();

    assert!(out.is_object(), "models info should return an object");
    for field in &["id", "author", "tags", "pipeline_tag", "library_name"] {
        assert!(out.get(*field).is_some(), "gpt2 info should have '{field}' field");
    }
}

// --- Datasets field structure tests ---

#[test]
fn datasets_list_json_has_expected_fields() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["datasets", "list", "--limit", "5"]).unwrap();

    let items = out.as_array().expect("datasets list should return an array");
    assert!(!items.is_empty(), "datasets list should return results");
    for item in items {
        assert!(item.get("id").is_some(), "dataset item should have 'id' field");
        assert!(item.get("author").is_some(), "dataset item should have 'author' field");
        assert!(item.get("tags").is_some(), "dataset item should have 'tags' field");
    }
}

#[test]
fn datasets_list_with_search() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs
        .run_json(&["datasets", "list", "--search", "squad", "--limit", "3"])
        .unwrap();

    let items = out.as_array().expect("datasets list should return an array");
    assert!(!items.is_empty(), "datasets search should return results");
}

// --- Spaces field structure tests ---

#[test]
fn spaces_list_json_has_expected_fields() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["spaces", "list", "--limit", "5"]).unwrap();

    let items = out.as_array().expect("spaces list should return an array");
    assert!(!items.is_empty(), "spaces list should return results");
    for item in items {
        assert!(item.get("id").is_some(), "space item should have 'id' field");
        assert!(item.get("author").is_some(), "space item should have 'author' field");
        assert!(item.get("sdk").is_some(), "space item should have 'sdk' field");
    }
}

// --- Papers tests ---

#[test]
fn papers_info_returns_valid_paper() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["papers", "info", "1706.03762"]).unwrap();

    assert!(out.is_object(), "papers info should return an object");
    assert!(out.get("id").is_some(), "paper should have 'id' field");
    assert!(out.get("title").is_some(), "paper should have 'title' field");
    assert!(out.get("authors").is_some(), "paper should have 'authors' field");
}

#[test]
fn papers_list_with_date_filter() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs
        .run_json(&["papers", "list", "--date", "2024-01-15", "--limit", "3"])
        .unwrap();

    assert!(out.is_array(), "papers list should return an array");
}

// --- Auth tests ---

#[test]
fn auth_whoami_has_username() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["auth", "whoami"]).unwrap();

    assert!(out.is_object(), "whoami should return an object");
    let username = out
        .get("username")
        .and_then(|v| v.as_str())
        .expect("whoami should have username field");
    assert!(!username.is_empty(), "username should not be empty");
}

// --- Repos tag list tests ---

#[test]
fn repos_tag_list_gpt2() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_json(&["repos", "tag", "list", "gpt2"]).unwrap();

    assert!(out.is_array(), "repos tag list should return an array");
}

// --- Collections tests ---

#[test]
fn collections_list_with_owner() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs
        .run_json(&["collections", "list", "--owner", "huggingface", "--limit", "3"])
        .unwrap();

    assert!(out.is_array(), "collections list should return an array");
    assert!(!out.as_array().unwrap().is_empty(), "collections list with owner should return results");
}

// --- Comparison tests ---

#[test]
fn models_list_sort_comparison() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs
        .run_json(&["models", "list", "--sort", "downloads", "--limit", "3"])
        .unwrap();
    let hf_out = hf.run_json(&["models", "list", "--sort", "downloads", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
    assert_eq!(
        hfrs_out.as_array().unwrap().len(),
        hf_out.as_array().unwrap().len(),
        "both CLIs should return the same number of results when sorting by downloads"
    );
}

#[test]
fn datasets_list_with_search_matches_hf() {
    require_token();
    let hfrs = CliRunner::hfrs();
    let hf = CliRunner::new("hf");
    require_cli(&hf);

    let hfrs_out = hfrs
        .run_json(&["datasets", "list", "--search", "squad", "--limit", "3"])
        .unwrap();
    let hf_out = hf.run_json(&["datasets", "list", "--search", "squad", "--limit", "3"]).unwrap();

    assert!(hfrs_out.is_array());
    assert!(hf_out.is_array());
    assert_eq!(
        hfrs_out.as_array().unwrap().len(),
        hf_out.as_array().unwrap().len(),
        "both CLIs should return the same number of results for squad search"
    );
}

// --- Output format tests ---

#[test]
fn datasets_list_table_has_headers() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs
        .run_raw(&["datasets", "list", "--limit", "3", "--format", "table"])
        .unwrap();

    assert!(out.contains("ID"), "datasets table should have 'ID' header");
    assert!(out.contains("Author"), "datasets table should have 'Author' header");
}

#[test]
fn spaces_list_quiet_output() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs.run_raw(&["spaces", "list", "--limit", "3", "--quiet"]).unwrap();

    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines.len(), 3, "quiet mode should output 3 IDs");
    for line in &lines {
        assert!(!line.contains(' '), "quiet mode lines should be plain IDs, got: '{line}'");
    }
}

#[test]
fn papers_search_quiet_output() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let out = hfrs
        .run_raw(&["papers", "search", "transformer", "--limit", "3", "--quiet"])
        .unwrap();

    let lines: Vec<&str> = out.trim().lines().collect();
    assert!(!lines.is_empty(), "quiet papers search should output IDs");
    for line in &lines {
        assert!(!line.is_empty(), "quiet mode lines should not be empty");
    }
}

// --- Error handling tests ---

#[test]
fn models_info_nonexistent_fails() {
    require_token();
    let hfrs = CliRunner::hfrs();

    let (code, _stderr) = hfrs
        .run_expecting_failure(&["models", "info", "nonexistent-model-xyz-12345"])
        .unwrap();

    assert_ne!(code, 0, "models info on nonexistent model should exit with non-zero code");
}

// --- Write tests ---

#[test]
fn write_repo_create_and_delete() {
    require_token();
    require_write();
    let hfrs = CliRunner::hfrs();

    let repo_name = format!(
        "hfrs-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let create_result = hfrs.run_raw(&["repos", "create", &repo_name, "--type", "model"]);
    assert!(create_result.is_ok(), "repo creation should succeed: {:?}", create_result.err());

    let full_repo = format!("assafvayner/{repo_name}");
    let info_result = hfrs.run_json(&["models", "info", &full_repo]);
    let delete_result = hfrs.run_raw(&["repos", "delete", &full_repo]);

    assert!(info_result.is_ok(), "newly created repo should be retrievable via models info");
    assert!(delete_result.is_ok(), "repo deletion should succeed: {:?}", delete_result.err());
}

#[test]
fn write_repo_create_private() {
    require_token();
    require_write();
    let hfrs = CliRunner::hfrs();

    let repo_name = format!(
        "hfrs-test-private-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let create_result = hfrs.run_raw(&["repos", "create", &repo_name, "--type", "model", "--private"]);
    assert!(create_result.is_ok(), "private repo creation should succeed: {:?}", create_result.err());

    let full_repo = format!("assafvayner/{repo_name}");
    let info_result = hfrs.run_json(&["models", "info", &full_repo]);
    let delete_result = hfrs.run_raw(&["repos", "delete", &full_repo]);

    let info = info_result.expect("private repo info should be retrievable by owner");
    assert_eq!(info.get("private").and_then(|v| v.as_bool()), Some(true), "repo should be private");
    assert!(delete_result.is_ok(), "private repo deletion should succeed: {:?}", delete_result.err());
}

#[test]
fn write_branch_create_and_delete() {
    require_token();
    require_write();
    let hfrs = CliRunner::hfrs();

    let repo_name = format!(
        "hfrs-test-branch-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let full_repo = format!("assafvayner/{repo_name}");

    hfrs.run_raw(&["repos", "create", &repo_name, "--type", "model"])
        .expect("repo creation should succeed");

    let branch_result = hfrs.run_raw(&["repos", "branch", "create", &full_repo, "test-branch"]);
    assert!(branch_result.is_ok(), "branch creation should succeed: {:?}", branch_result.err());

    let delete_branch_result = hfrs.run_raw(&["repos", "branch", "delete", &full_repo, "test-branch"]);
    assert!(delete_branch_result.is_ok(), "branch deletion should succeed: {:?}", delete_branch_result.err());

    let delete_repo_result = hfrs.run_raw(&["repos", "delete", &full_repo]);
    assert!(delete_repo_result.is_ok(), "repo deletion should succeed: {:?}", delete_repo_result.err());
}

#[test]
fn write_tag_create_and_delete() {
    require_token();
    require_write();
    let hfrs = CliRunner::hfrs();

    let repo_name = format!(
        "hfrs-test-tag-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let full_repo = format!("assafvayner/{repo_name}");

    hfrs.run_raw(&["repos", "create", &repo_name, "--type", "model"])
        .expect("repo creation should succeed");

    let tag_result = hfrs.run_raw(&["repos", "tag", "create", &full_repo, "v0.1"]);
    let list_result = hfrs.run_json(&["repos", "tag", "list", &full_repo]);
    let delete_tag_result = hfrs.run_raw(&["repos", "tag", "delete", &full_repo, "v0.1"]);
    let delete_repo_result = hfrs.run_raw(&["repos", "delete", &full_repo]);

    assert!(tag_result.is_ok(), "tag creation should succeed: {:?}", tag_result.err());

    let tags = list_result.expect("tag list should succeed");
    let empty = vec![];
    let tag_names: Vec<&str> = tags
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(tag_names.contains(&"v0.1"), "tag list should contain 'v0.1', got: {tag_names:?}");

    assert!(delete_tag_result.is_ok(), "tag deletion should succeed: {:?}", delete_tag_result.err());
    assert!(delete_repo_result.is_ok(), "repo deletion should succeed: {:?}", delete_repo_result.err());
}

#[test]
fn write_discussion_create_and_close() {
    require_token();
    require_write();
    let hfrs = CliRunner::hfrs();

    let repo_name = format!(
        "hfrs-test-disc-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let full_repo = format!("assafvayner/{repo_name}");

    hfrs.run_raw(&["repos", "create", &repo_name, "--type", "model"])
        .expect("repo creation should succeed");

    // Upload a README so the repo has an initial commit (required for discussions)
    let tmp_dir = std::env::temp_dir().join(format!("hfrs-disc-test-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let readme_path = tmp_dir.join("README.md");
    std::fs::write(&readme_path, "# Test repo\n").unwrap();
    hfrs.run_raw(&["upload", &full_repo, readme_path.to_str().unwrap(), "README.md"])
        .expect("README upload should succeed");
    let _ = std::fs::remove_dir_all(&tmp_dir);

    // Retry discussion creation — Hub may need time to index the repo
    let mut disc_result = None;
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        let result = hfrs.run_raw(&["discussions", "create", &full_repo, "--title", "Test Discussion"]);
        if result.is_ok() {
            disc_result = Some(result.unwrap());
            break;
        }
        if attempt == 2 {
            // Cleanup and fail
            let _ = hfrs.run_raw(&["repos", "delete", &full_repo]);
            panic!("discussion creation failed after 3 attempts: {:?}", result.err());
        }
    }

    let disc_num = disc_result.unwrap().trim().to_string();

    let close_result = hfrs.run_raw(&["discussions", "close", &full_repo, &disc_num]);
    assert!(close_result.is_ok(), "discussion close should succeed: {:?}", close_result.err());

    let info_result = hfrs.run_json(&["discussions", "info", &full_repo, &disc_num]);
    let info_obj = info_result.expect("discussion info should succeed after close");
    let status = info_obj.get("status").and_then(|s| s.as_str()).unwrap_or("");
    assert_eq!(status, "closed", "discussion should be closed, got: '{status}'");

    let delete_result = hfrs.run_raw(&["repos", "delete", &full_repo]);
    assert!(delete_result.is_ok(), "repo deletion should succeed: {:?}", delete_result.err());
}
