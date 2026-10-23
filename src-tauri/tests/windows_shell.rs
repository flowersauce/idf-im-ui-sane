#![cfg(all(feature = "cli", target_os = "windows"))]

use serde_json::json;
use std::fs;
use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn activated_shell(parent: Option<&str>, explicit: Option<&str>, profile: bool) -> Output {
    let temp = TempDir::new().unwrap();
    let activation = temp.path().join("O'Brien $literal 登记.ps1");
    fs::write(
        &activation,
        "function Eim-SessionMarker { 'EIM_ACTIVATED_IN_SESSION' }\nSet-Alias Eim-SessionAlias Eim-SessionMarker\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("eim_idf.json"),
        serde_json::to_vec(&json!({
            "gitPath": "git",
            "idfInstalled": [{
                "id": "test", "name": "test", "path": temp.path(),
                "idfToolsPath": temp.path(), "activationScript": activation,
            }],
            "idfSelectedId": "test",
        }))
        .unwrap(),
    )
    .unwrap();

    let mut command = if let Some(parent) = parent {
        let mut command = Command::new(parent);
        let choice = if explicit.is_some() {
            " --shell $env:EIM_SHELL_TEST_CHOICE"
        } else {
            ""
        };
        command.args([
            "-NoLogo",
            "-NoProfile",
            "-Command",
            &format!("& $env:EIM_SHELL_TEST_BIN --do-not-track=true --log-file $env:EIM_SHELL_TEST_LOG shell{choice}; exit $LASTEXITCODE"),
        ]);
        command
    } else {
        let mut command = Command::new(env!("CARGO_BIN_EXE_eim"));
        command.args(["--do-not-track=true", "--log-file"]);
        command.arg(temp.path().join("eim.log")).arg("shell");
        if let Some(executable) = explicit {
            command.args(["--shell", executable]);
        }
        command
    };
    command
        .env("EIM_ESP_IDF_JSON_PATH", temp.path())
        .env("EIM_SHELL_TEST_BIN", env!("CARGO_BIN_EXE_eim"))
        .env("EIM_SHELL_TEST_LOG", temp.path().join("eim.log"))
        .env("EIM_SHELL_TEST_CHOICE", explicit.unwrap_or_default())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    {
        let mut input = child.stdin.take().unwrap();
        writeln!(input, "Eim-SessionAlias").unwrap();
        writeln!(
            input,
            "Write-Output ('EIM_PS_MAJOR=' + $PSVersionTable.PSVersion.Major)"
        )
        .unwrap();
        if profile {
            writeln!(input, "Eim-ProfileMarker").unwrap();
            writeln!(input, "Write-Output ('EIM_PROMPT=' + (prompt))").unwrap();
        }
        writeln!(input, "exit 0").unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            // Terminate the entire test-owned tree; killing just the parent
            // PowerShell could leave eim and its interactive child holding pipes.
            let _ = Command::new("taskkill")
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .output();
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!(
                "Interactive shell timed out:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EIM_ACTIVATED_IN_SESSION"), "{stdout}");
    if profile {
        assert!(stdout.contains("EIM_PROFILE_LOADED"), "{stdout}");
        assert!(
            stdout.contains("EIM_PROMPT=EIM_PROFILE_PROMPT>"),
            "{stdout}"
        );
    }
    output
}

#[test]
fn preserves_invoking_powershell_7_and_51() {
    for (parent, expected) in [("pwsh", "EIM_PS_MAJOR=7"), ("powershell", "EIM_PS_MAJOR=5")] {
        let output = activated_shell(Some(parent), None, false);
        assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
    }
}

#[test]
fn explicit_choice_overrides_the_invoking_powershell() {
    for (parent, explicit, expected) in [
        ("pwsh", "powershell", "EIM_PS_MAJOR=5"),
        ("powershell", "pwsh", "EIM_PS_MAJOR=7"),
    ] {
        let output = activated_shell(Some(parent), Some(explicit), false);
        assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
    }
}

#[test]
fn explicit_full_executable_path_is_supported() {
    let output = Command::new("pwsh")
        .args(["-NoProfile", "-Command", "(Get-Process -Id $PID).Path"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let path = String::from_utf8(output.stdout).unwrap();
    let output = activated_shell(None, Some(path.trim()), false);
    assert!(String::from_utf8_lossy(&output.stdout).contains("EIM_PS_MAJOR=7"));
}

#[test]
#[ignore = "requires the test profiles prepared by windows-integration.yml"]
fn loads_profiles_and_preserves_their_prompt_in_both_powershells() {
    for executable in ["pwsh", "powershell"] {
        activated_shell(None, Some(executable), true);
    }
}
