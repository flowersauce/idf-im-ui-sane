#![cfg(feature = "cli")]

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn registry(root: &Path, directory: &str, names: &[&str]) -> PathBuf {
    let path = root.join(directory);
    fs::create_dir_all(&path).unwrap();
    let installations: Vec<_> = names
        .iter()
        .map(|name| {
            let idf_path = root.join(directory).join(name).join("esp-idf");
            fs::create_dir_all(&idf_path).unwrap();
            json!({
                "id": name,
                "name": name,
                "path": idf_path,
                "idfToolsPath": root.join("tools"),
            })
        })
        .collect();
    fs::write(
        path.join("eim_idf.json"),
        serde_json::to_vec(&json!({
            "gitPath": "git",
            "idfInstalled": installations,
            "idfSelectedId": names[0],
        }))
        .unwrap(),
    )
    .unwrap();
    path
}

fn eim(root: &Path, environment: &Path, args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_eim"))
        .env("EIM_ESP_IDF_JSON_PATH", environment)
        .args(["--do-not-track=true", "--log-file"])
        .arg(root.join("eim.log"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "eim {args:?} failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn read_registry(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path.join("eim_idf.json")).unwrap()).unwrap()
}

#[test]
fn environment_registry_is_shared_by_management_commands() {
    let temp = TempDir::new().unwrap();
    let path = registry(temp.path(), "registry with spaces 登记", &["alpha", "beta"]);

    let list = eim(temp.path(), &path, &["list"]);
    let text = String::from_utf8_lossy(&list.stdout);
    assert!(text.contains("alpha") && text.contains("beta"));

    eim(temp.path(), &path, &["select", "beta"]);
    assert_eq!(read_registry(&path)["idfSelectedId"], "beta");

    eim(temp.path(), &path, &["rename", "beta", "renamed"]);
    assert_eq!(read_registry(&path)["idfInstalled"][1]["name"], "renamed");

    eim(temp.path(), &path, &["remove", "alpha"]);
    let remaining = read_registry(&path);
    assert_eq!(remaining["idfInstalled"].as_array().unwrap().len(), 1);
    assert_eq!(remaining["idfInstalled"][0]["id"], "beta");
}

#[test]
fn explicit_registry_overrides_environment_before_or_after_subcommand() {
    let temp = TempDir::new().unwrap();
    let environment = registry(temp.path(), "environment", &["environment-only"]);
    let explicit = registry(temp.path(), "explicit", &["explicit-only"]);
    let explicit = explicit.to_str().unwrap();

    for args in [
        vec!["--esp-idf-json-path", explicit, "list"],
        vec!["list", "--esp-idf-json-path", explicit],
    ] {
        let output = eim(temp.path(), &environment, &args);
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("explicit-only"));
        assert!(!text.contains("environment-only"));
    }
}

#[test]
fn settings_registry_precedence_and_empty_environment() {
    // Run environment-dependent library checks in child processes, so other
    // parallel tests never observe a temporary process-wide environment change.
    if let Some(root) = std::env::var_os("EIM_REGISTRY_TEST_ROOT") {
        use idf_im_lib::settings::Settings;
        let root = PathBuf::from(root);
        let expected_default = std::env::var("EIM_REGISTRY_TEST_DEFAULT").unwrap();
        let expected_loaded = std::env::var("EIM_REGISTRY_TEST_LOADED").unwrap();
        assert_eq!(
            Settings::default().esp_idf_json_path,
            Some(expected_default.clone())
        );
        assert_eq!(
            idf_im_lib::version_manager::get_default_config_path(),
            PathBuf::from(expected_default).join("eim_idf.json")
        );

        let config = Some(root.join("config.toml"));
        let loaded = Settings::new(config.clone(), []).unwrap();
        assert_eq!(loaded.esp_idf_json_path, Some(expected_loaded));
        let explicit = root.join("explicit registry").to_str().unwrap().to_string();
        let loaded = Settings::new(
            config,
            [(
                "esp_idf_json_path".into(),
                Some(config::Value::from(explicit.clone())),
            )],
        )
        .unwrap();
        assert_eq!(loaded.esp_idf_json_path, Some(explicit));
        return;
    }

    let temp = TempDir::new().unwrap();
    let file_path = temp
        .path()
        .join("file registry")
        .to_str()
        .unwrap()
        .to_string();
    let mut settings = idf_im_lib::settings::Settings::default();
    let legacy_default = settings.tool_install_folder_name.clone().unwrap();
    settings.esp_idf_json_path = Some(file_path.clone());
    fs::write(
        temp.path().join("config.toml"),
        toml::to_string(&settings).unwrap(),
    )
    .unwrap();
    let environment_path = temp
        .path()
        .join("environment 登记")
        .to_str()
        .unwrap()
        .to_string();

    for environment in [
        None,
        Some(""),
        Some("  \t"),
        Some(environment_path.as_str()),
    ] {
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "settings_registry_precedence_and_empty_environment",
                "--nocapture",
            ])
            .env_remove("EIM_ESP_IDF_JSON_PATH")
            .env("EIM_REGISTRY_TEST_ROOT", temp.path());
        if let Some(value) = environment {
            child.env("EIM_ESP_IDF_JSON_PATH", value);
        }
        let has_override = environment == Some(environment_path.as_str());
        child
            .env(
                "EIM_REGISTRY_TEST_DEFAULT",
                if has_override {
                    &environment_path
                } else {
                    &legacy_default
                },
            )
            .env(
                "EIM_REGISTRY_TEST_LOADED",
                if has_override {
                    &environment_path
                } else {
                    &file_path
                },
            );
        let output = child.output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
