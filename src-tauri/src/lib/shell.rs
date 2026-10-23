use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

fn is_powershell(path: &Path) -> bool {
    // Recognize Windows paths in platform-independent tests as well.
    let path = path.as_os_str().to_string_lossy();
    let name = path.rsplit(['/', '\\']).next().unwrap_or_default();
    ["pwsh", "pwsh.exe", "powershell", "powershell.exe"]
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn invoking_powershell() -> Option<PathBuf> {
    let mut system = System::new();
    let mut pid = sysinfo::get_current_pid().ok()?;
    let mut visited = HashSet::new();
    // Only inspect executable paths and parent IDs, not arguments or environment.
    for _ in 0..32 {
        if !visited.insert(pid) {
            break;
        }
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            false,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );
        let process = system.process(pid)?;
        if let Some(exe) = process.exe().filter(|exe| is_powershell(exe)) {
            return Some(exe.to_path_buf());
        }
        pid = process.parent()?;
    }
    None
}

fn select_powershell(
    explicit: Option<&Path>,
    parent: Option<PathBuf>,
    fallback: impl FnOnce() -> PathBuf,
) -> Result<PathBuf> {
    if let Some(executable) = explicit {
        if !is_powershell(executable) {
            bail!("On Windows, --shell must select pwsh or powershell (or the full path to either executable)");
        }
        return Ok(executable.to_path_buf());
    }
    Ok(parent.unwrap_or_else(fallback))
}

fn powershell_command(executable: &Path, activation_script: &str) -> Command {
    // Dot-source in the final interactive process. A literal PowerShell string
    // keeps spaces, apostrophes, dollar signs and Unicode paths intact.
    let script = format!(". '{}'", activation_script.replace('\'', "''"));
    let mut command = Command::new(executable);
    command.args(["-NoLogo", "-NoExit", "-Command", &script]);
    // Inherit the terminal and load the selected PowerShell's normal profiles.
    // The noninteractive WindowsExecutor deliberately suppresses both.
    command
}

pub(crate) fn run_windows_shell(
    activation_script: &str,
    explicit: Option<&Path>,
) -> Result<ExitStatus> {
    let parent = if explicit.is_none() {
        invoking_powershell()
    } else {
        None
    };
    let executable = select_powershell(explicit, parent, default_powershell)?;
    log::debug!("Starting interactive PowerShell: {}", executable.display());
    powershell_command(&executable, activation_script)
        .status()
        .with_context(|| format!("Failed to start interactive shell {}", executable.display()))
}

fn default_powershell() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        crate::command_executor::find_powershell()
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("powershell")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_shell_wins_over_parent_and_fallback() {
        for executable in ["pwsh", "powershell", r"C:\Portable Tools\PWSH.EXE"] {
            let chosen = select_powershell(
                Some(Path::new(executable)),
                Some(PathBuf::from("parent/powershell.exe")),
                || panic!("An explicit selection must not fall back"),
            )
            .unwrap();
            assert_eq!(chosen, Path::new(executable));
        }
        assert!(select_powershell(Some(Path::new("cmd.exe")), None, || unreachable!()).is_err());
    }

    #[test]
    fn parent_shell_is_preserved_before_discovery() {
        for parent in [r"C:\PowerShell\7\pwsh.exe", r"C:\Windows\powershell.exe"] {
            let chosen = select_powershell(None, Some(PathBuf::from(parent)), || {
                panic!("An identified parent must not fall back")
            })
            .unwrap();
            assert_eq!(chosen, Path::new(parent));
        }
        assert_eq!(
            select_powershell(None, None, || PathBuf::from("discovered/pwsh.exe")).unwrap(),
            Path::new("discovered/pwsh.exe")
        );
    }

    #[test]
    fn activation_is_literal_and_runs_in_the_interactive_shell() {
        let path = r"C:\ESP tools\O'Brien $env\登记.ps1";
        let command = powershell_command(Path::new("pwsh"), path);
        assert_eq!(command.get_program(), "pwsh");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.len(), 4);
        assert_eq!(args[..3], ["-NoLogo", "-NoExit", "-Command"]);
        assert_eq!(args[3], r". 'C:\ESP tools\O''Brien $env\登记.ps1'");
    }

    #[test]
    fn powershell_detection_requires_the_actual_executable_name() {
        assert!(is_powershell(Path::new(r"C:\portable\PwSh.ExE")));
        assert!(is_powershell(Path::new("/tools/powershell.exe")));
        assert!(!is_powershell(Path::new("pwsh-wrapper.exe")));
        assert!(!is_powershell(Path::new("cmd.exe")));
    }
}
