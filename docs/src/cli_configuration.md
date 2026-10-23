# CLI Configuration

> **Important:** CLI releases (e.g., `eim-cli-*.exe` on Windows) must be run from a terminal like PowerShell rather than double-clicking the executable. Double-clicking will open a terminal window briefly, display help information, and then close immediately, which may appear as if the installer crashed.

EIM does not add its own directory to `PATH` at startup. If you download the Windows executable directly, run it by path, or rename it to `eim.exe` and add its directory to your user `PATH` to use `eim` from any terminal.

The command-line interface supports multiple configuration methods with the following priority (highest to lowest):

1. Command line arguments
2. Environment variables
3. Configuration files
4. Default values

## Command Structure

ESP-IDF Installation Manager (EIM) now uses a command-based structure with the following format:

```bash
eim [OPTIONS] [COMMAND] [COMMAND_OPTIONS]
```

For example:
```bash
# Install ESP-IDF with specific version
eim install -i v5.3.2

# Run the interactive wizard
eim wizard

# List installed versions
eim list
```

For a complete list of available commands and their options, see [CLI Commands](./cli_commands.md).

## Command Line Arguments

View all available options with:
```bash
eim --help
```

For help with a specific command:
```bash
eim <command> --help
```

## Environment Variables

### Installation registry directory

Set `EIM_ESP_IDF_JSON_PATH` to the **directory containing** `eim_idf.json` to choose a persistent registry location. Installation, wizard, GUI startup, and version-management commands respect the same setting.

```powershell
# Current PowerShell session and its child processes
$env:EIM_ESP_IDF_JSON_PATH = Join-Path $env:LOCALAPPDATA 'Espressif/tools'
eim list
eim shell

# Persist for future sessions (reopen your terminal afterward)
[Environment]::SetEnvironmentVariable('EIM_ESP_IDF_JSON_PATH', $env:EIM_ESP_IDF_JSON_PATH, 'User')
```

On POSIX systems, use `export EIM_ESP_IDF_JSON_PATH="$HOME/.local/share/espressif/tools"`.

An explicit `--esp-idf-json-path` wins over the environment variable, which wins over the `esp_idf_json_path` setting in an installation configuration file. Empty or whitespace-only environment values are ignored. Without an override, the existing platform default is unchanged. This setting selects the registry; it does not move existing registrations, ESP-IDF installations, or tool directories. Point it at an existing registry or use it when creating new installations.

### Other settings

For other installation settings, use the command-line arguments or TOML configuration files described below.

## Configuration Files

> **Note on Python versions:** ESP-IDF supports Python versions 3.10, 3.11, 3.12, 3.13, and 3.14. Python 3.14 is now fully supported on Linux, macOS, and Windows.

Use TOML format configuration files for reproducible installations:

```toml
path = "/Users/testusername/.espressif"
idf_path = "/Users/testusername/.espressif/v5.5/esp-idf"
esp_idf_json_path = "/Users/testusername/.espressif/tools"
tool_download_folder_name = "/Users/testusername/.espressif/dist"
tool_install_folder_name = "/Users/testusername/.espressif/tools"
activation_script_path_override = "/Users/testusername/.espressif/tools"
create_bat_activation_script = false
python_env_folder_name = "python_env"
cleanup = true
target = ["all"]
idf_versions = ["v5.5"]
tools_json_file = "tools/tools.json"
config_file_save_path = "eim_config.toml"
non_interactive = true
wizard_all_questions = false
mirror = "https://github.com"
idf_mirror = "https://github.com"
pypi_mirror = "https://pypi.org/simple"
recurse_submodules = true
install_all_prerequisites = true
skip_prerequisites_check = false
skip_components_download = false
idf_features = ["ci", "docs"]
```

Load a configuration file:
```bash
eim install --config path/to/config.toml
```

## IDF Features Configuration

ESP-IDF supports optional features (such as `ci`, `docs`, `pytest`, etc.) that install additional Python dependencies. You can configure these features in several ways:

### Global Features (All Versions)

Use the `--idf-features` flag or `idf_features` config option to apply the same features to all ESP-IDF versions being installed:

```bash
# Via command line
eim install -i v5.3.2,v5.4 --idf-features=ci,docs

# Via configuration file
idf_features = ["ci", "docs", "pytest"]
```

### Per-Version Features

When installing multiple ESP-IDF versions, you may want different features for each version. Use the `idf_features_per_version` configuration option in your TOML file:

```toml
idf_versions = ["v5.3.2", "v5.4", "v5.5"]

# Per-version feature selection
[idf_features_per_version]
"v5.3.2" = ["ci", "docs"]
"v5.4" = ["ci", "pytest"]
"v5.5" = ["ci", "docs", "pytest", "sbom"]
```

### Feature Selection Priority

The installer determines which features to use for each version in the following order:

1. **Per-version features** (`idf_features_per_version`): If specified for the version, these are used
2. **Global features** (`idf_features` or `--idf-features`): Applied to all versions without per-version settings
3. **Interactive selection**: In wizard mode, you'll be prompted to select features for each version
4. **Required only**: In non-interactive mode without any feature configuration, only required features are installed

## IDF Tools Configuration

Similar to features, ESP-IDF supports configuring which development tools to install. You can configure these tools in several ways:

### Global Tools (All Versions)

Use the `--idf-tools` flag or `idf_tools` config option to apply the same tools to all ESP-IDF versions being installed:

```bash
# Via command line
eim install -i v5.3.2,v5.4 --idf-tools=cmake,openocd

# Via configuration file
idf_tools = ["cmake", "openocd", "idf-exe"]
```

### Per-Version Tools

When installing multiple ESP-IDF versions, you may want different tools for each version. Use the `idf_tools_per_version` configuration option in your TOML file:

```toml
idf_versions = ["v5.3.2", "v5.4", "v5.5"]

# Per-version tool selection
[idf_tools_per_version]
"v5.3.2" = ["cmake", "openocd"]
"v5.4" = ["cmake", "openocd", "idf-exe"]
"v5.5" = ["cmake", "openocd", "idf-exe", "esp-venv"]
```

### Tool Selection Priority

The installer determines which tools to use for each version in the following order:

1. **Per-version tools** (`idf_tools_per_version`): If specified for the version, these are used
2. **Global tools** (`idf_tools` or `--idf-tools`): Applied to all versions without per-version settings
3. **Interactive selection**: In wizard mode, you'll be prompted to select tools for each version
4. **Required only**: In non-interactive mode without any tool configuration, only required tools are installed

### Changing Tools or Features on an Existing Installation

The `eim fix` command (see [CLI Commands](./cli_commands.md#fix-command)) accepts the same `--idf-features` / `--idf-tools` (and per-version equivalents) as `install`/`wizard`. By default it reinstalls using the configuration the version was originally installed with, so nothing is lost. Passing `--idf-features`/`--idf-tools` explicitly to `eim fix` overrides the preserved configuration for that run, letting you add (or change) tools/features without redoing the whole installation:

```bash
eim fix -p /path/to/existing/esp-idf --idf-tools cmake,openocd
```

Use [`eim list-tools`](./cli_commands.md#list-tools-command) and [`eim list-features`](./cli_commands.md#list-features-command) to see what's currently installed for a version before deciding what to add.

### Interactive Tool Selection

When using the wizard command, you'll be prompted to select tools for each ESP-IDF version:

```bash
eim wizard -i v5.3.2,v5.4
```

The wizard will:
- Fetch available tools for each version and your platform
- Display required tools (pre-selected and cannot be deselected)
- Allow you to select/deselect optional tools independently for each version
- Save your selections to the configuration if you choose to export it

### Interactive Feature Selection

When using the wizard command, you'll be prompted to select features for each ESP-IDF version:

```bash
eim wizard -i v5.3.2,v5.4
```

The wizard will:
- Display available features for each version (features may differ between versions)
- Show which features are required vs optional
- Allow you to select/deselect optional features independently for each version
- Save your selections to the configuration if you choose to export it

Load a configuration file:
```bash
eim install --config path/to/config.toml
```

## Headless Configuration

For automated installations, use the `install` command which runs in non-interactive mode by default:

```bash
# Basic headless installation
eim install

# Headless with specific version and path
eim install -i v5.3.2 -p /opt/esp-idf

# Headless with config file
eim install --config path/to/config.toml

# Headless with specific features
eim install -i v5.3.2 --idf-features=ci,docs

# To run in interactive mode, explicitly set non-interactive to false
eim install -n false
```

See [Headless Usage](./headless_usage.md) for more details about automated installations.
