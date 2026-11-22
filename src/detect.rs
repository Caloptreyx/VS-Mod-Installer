//! Pure detection logic for Vintage Story servers: whether a server runs Vintage Story, where its
//! data path and mods folder are, and which game version it runs. The route in
//! `routes/server.rs` gathers the inputs from Wings and the database.
//!
//! Paths are relative to the server root (`/home/container` inside the container), without a
//! leading slash; `""` is the root itself.
use crate::json;
use serde::Serialize;
use serde_json::Value;

/// Where a value was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The admin restricted the extension to the server's egg.
    Settings,
    /// The egg, nest or Docker image name.
    Egg,
    /// The startup command.
    Startup,
    /// `serverconfig.json` in the data path.
    Config,
    /// The game's default when nothing else says otherwise.
    Default,
    /// Files in the server root (the game binaries, or `.vs-version` written by the Caloptreyx
    /// egg's installer).
    Files,
    /// `server-main.log`.
    Log,
    /// A startup variable such as `VS_VERSION`.
    Variables,
}

/// Where the game keeps its data without `--dataPath`: .NET's ApplicationData folder, which is
/// `~/.config` on Linux, and the container's home is the server root.
pub const DEFAULT_DATA_PATH: &str = ".config/VintagestoryData";
/// The game's own mods (`VSEssentials.dll`, ...) live in `Mods` next to the binaries, and the
/// game reads that folder first. User mods never go there.
const BINARIES_MODS: &str = "Mods";
const CONTAINER_HOME: &str = "/home/container";

/// Files in the server root that only a Vintage Story server has.
pub const SERVER_FILES: [&str; 4] = [
    "VintagestoryServer.dll",
    "VintagestoryServer",
    "VintagestoryAPI.dll",
    "VintagestoryLib.dll",
];
/// The game's own mod files, which sit in the binaries' `Mods` folder.
pub const BUILTIN_MOD_FILES: [&str; 3] =
    ["VSEssentials.dll", "VSSurvivalMod.dll", "VSCreativeMod.dll"];
/// Written by the Caloptreyx Vintage Story egg's installer: the installed version.
pub const INSTALL_VERSION_FILE: &str = ".vs-version";
const VERSION_VARIABLES: [&str; 7] = [
    "VS_VERSION",
    "VINTAGE_STORY_VERSION",
    "VINTAGESTORY_VERSION",
    "RELEASE_VERSION",
    "GAME_VERSION",
    "SERVER_VERSION",
    "VERSION",
];

/// Whether a server runs Vintage Story, and what gave it away.
pub fn is_vintage_story(
    egg_name: &str,
    nest_name: &str,
    image: &str,
    startup: &str,
    root_files: &[String],
) -> Option<Source> {
    if root_files
        .iter()
        .any(|name| SERVER_FILES.contains(&name.as_str()))
    {
        return Some(Source::Files);
    }
    if startup.to_ascii_lowercase().contains("vintagestory") {
        return Some(Source::Startup);
    }
    let names = format!("{egg_name}\n{nest_name}\n{image}").to_ascii_lowercase();
    if names.contains("vintage story")
        || names.contains("vintagestory")
        || names.contains("vintage_story")
        || names.contains("vintage-story")
    {
        return Some(Source::Egg);
    }
    None
}

/// Replaces `{{NAME}}` and `${NAME}` with the startup variables' values; unknown names stay.
pub fn substitute(command: &str, variables: &[(String, String)]) -> String {
    let lookup = |name: &str| {
        let name = name.trim();
        let name = name.strip_prefix("env.").unwrap_or(name);
        variables
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    let mut out = String::with_capacity(command.len());
    let mut rest = command;
    loop {
        let next = [rest.find("{{"), rest.find("${")]
            .into_iter()
            .flatten()
            .min();
        let Some(start) = next else {
            out.push_str(rest);
            return out;
        };
        let (open, close) = if rest[start..].starts_with("{{") {
            ("{{", "}}")
        } else {
            ("${", "}")
        };
        out.push_str(&rest[..start]);
        let after = &rest[start + open.len()..];
        match after.find(close) {
            Some(end) => {
                let name = &after[..end];
                match lookup(name) {
                    Some(value) => out.push_str(value),
                    None => {
                        out.push_str(open);
                        out.push_str(name);
                        out.push_str(close);
                    }
                }
                rest = &after[end + close.len()..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
}

/// Splits a command line like a POSIX shell would split words: whitespace separates, quotes
/// group, a backslash escapes outside single quotes.
pub fn tokenize(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut chars = command.chars();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (Some('\''), '\'') => quote = None,
            (Some('\''), _) => current.push(ch),
            (Some('"'), '"') => quote = None,
            (Some('"'), '\\') => match chars.next() {
                Some(next @ ('"' | '\\' | '$' | '`')) => current.push(next),
                Some(next) => {
                    current.push('\\');
                    current.push(next);
                }
                None => current.push('\\'),
            },
            (Some(_), _) => current.push(ch),
            (None, '\'' | '"') => {
                quote = Some(ch);
                in_token = true;
            }
            (None, '\\') => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
                in_token = true;
            }
            (None, ';' | '&' | '|') => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            (None, ch) if ch.is_whitespace() => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            (None, _) => {
                current.push(ch);
                in_token = true;
            }
        }
    }
    if in_token {
        tokens.push(current);
    }
    tokens
}

/// The first value of `--name value` or `--name=value` (the game's option names, any case).
pub fn option_value(tokens: &[String], name: &str) -> Option<String> {
    let flag = format!("--{name}");
    let mut iter = tokens.iter();
    while let Some(token) = iter.next() {
        if token.eq_ignore_ascii_case(&flag) {
            return iter.next().cloned();
        }
        if let Some((key, value)) = token.split_once('=')
            && key.eq_ignore_ascii_case(&flag)
        {
            return Some(value.to_string());
        }
    }
    None
}

/// A path as the game process sees it (absolute, or relative to the server root, which is its
/// working directory) turned into a path relative to the server root. `None` for paths outside
/// the server's files.
pub fn container_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    let relative = if let Some(rest) = path
        .strip_prefix("~/")
        .or_else(|| path.strip_prefix("$HOME/"))
        .or_else(|| path.strip_prefix("${HOME}/"))
    {
        rest
    } else if path == "~" || path == "$HOME" || path == CONTAINER_HOME {
        ""
    } else if let Some(rest) = path.strip_prefix(CONTAINER_HOME) {
        rest.strip_prefix('/')?
    } else if path.starts_with('/') {
        return None;
    } else {
        path
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

pub fn join(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_string()
    } else {
        format!("{base}/{name}")
    }
}

/// The data path from `--dataPath` in the startup command, else the game's default.
pub fn data_path(startup: &str, variables: &[(String, String)]) -> (String, Source) {
    let tokens = tokenize(&substitute(startup, variables));
    match option_value(&tokens, "dataPath").and_then(|path| container_path(&path)) {
        Some(path) => (path, Source::Startup),
        None => (DEFAULT_DATA_PATH.to_string(), Source::Default),
    }
}

/// `server-main.log`: in `--logPath` when the startup command sets it, else in `<data>/Logs`.
pub fn main_log(startup: &str, variables: &[(String, String)], data_path: &str) -> String {
    let tokens = tokenize(&substitute(startup, variables));
    let logs = option_value(&tokens, "logPath")
        .and_then(|path| container_path(&path))
        .unwrap_or_else(|| join(data_path, "Logs"));
    join(&logs, "server-main.log")
}

/// The folder new mods go to: the first entry of `ModPaths` in `serverconfig.json` that is
/// inside the server's files and not the binaries' own `Mods` folder, else `<data>/Mods` (the
/// game's default entry).
pub fn mods_dir(data_path: &str, config: Option<&Value>, data_source: Source) -> (String, Source) {
    let default = join(data_path, "Mods");
    let configured = config
        .and_then(|config| json::get(config, "ModPaths"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(container_path)
        .find(|path| path != BINARIES_MODS || *path == default);
    match configured {
        Some(path) => (path, Source::Config),
        None => (default, data_source),
    }
}

/// `1.22.7` or `1.23.0-rc.2` (an optional leading `v` is dropped); `None` for anything else.
pub fn strict_version(text: &str) -> Option<String> {
    let text = text.trim();
    let text = text.strip_prefix('v').unwrap_or(text);
    let (core, suffix) = match text.split_once('-') {
        Some((core, suffix)) => (core, Some(suffix)),
        None => (text, None),
    };
    let numeric = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 || !parts.iter().all(|part| numeric(part)) {
        return None;
    }
    if let Some(suffix) = suffix {
        let (kind, number) = suffix.split_once('.')?;
        if !matches!(kind, "dev" | "pre" | "rc") || !numeric(number) {
            return None;
        }
    }
    Some(text.to_string())
}

/// The version from `Game Version: v1.22.7 (Stable)`, which the server logs on every start.
pub fn game_version_from_log(log: &str) -> Option<String> {
    log.lines().find_map(|line| {
        let (_, rest) = line.split_once("Game Version:")?;
        strict_version(rest.split_whitespace().next()?)
    })
}

/// A string of the version resource (`StringFileInfo`) of a DLL, by its key.
fn version_resource(dll: &[u8], key: &str) -> Option<String> {
    let key: Vec<u8> = key.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let at = dll.windows(key.len()).position(|window| window == key)? + key.len();
    // the key's terminator and padding, then the value up to its terminator
    let units: Vec<u16> = dll[at..]
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .skip_while(|&unit| unit == 0)
        .take_while(|&unit| unit != 0)
        .take(32)
        .collect();
    String::from_utf16(&units)
        .ok()
        .map(|value| value.trim().to_string())
}

/// The game version of a `VintagestoryAPI.dll`: its `Assembly Version` (`1.22.7.0` -> `1.22.7`),
/// when that belongs to the API version in `FileVersion` (`1.22.0`). 1.21 builds (checked with
/// 1.21.6) carry a constant `1.0.0.0` there, so they don't tell the patch and give `None`. No
/// pre-release suffix either way.
pub fn assembly_version(dll: &[u8]) -> Option<String> {
    let file = version_resource(dll, "FileVersion")?;
    let assembly = version_resource(dll, "Assembly Version")?;
    let file: Vec<&str> = file.split('.').collect();
    let parts: Vec<&str> = assembly.split('.').collect();
    if parts.len() < 3 || file.len() < 2 || parts[..2] != file[..2] {
        return None;
    }
    strict_version(&parts[..3].join("."))
}

/// A concrete version in a version startup variable (`latest` and the like don't count).
pub fn game_version_from_variables(variables: &[(String, String)]) -> Option<String> {
    VERSION_VARIABLES.iter().find_map(|name| {
        variables
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .and_then(|(_, value)| strict_version(value))
    })
}

/// Picks the game version from what was found.
///
/// The binaries are the ground truth for what runs at the next start: when the version of
/// `VintagestoryAPI.dll` is known, the egg's `.vs-version` or the log may only refine it with a
/// pre-release suffix (same `major.minor.patch`). A log from before an update is ignored that
/// way. Without the binaries, `.vs-version` comes before the log, and startup variables come
/// last.
pub fn pick_game_version(
    install_file: Option<String>,
    log: Option<String>,
    dll: Option<String>,
    variables: Option<String>,
) -> Option<(String, Source)> {
    let base = |version: &str| version.split('-').next().unwrap_or(version).to_string();
    match dll {
        Some(dll) => {
            let refined = [
                install_file.map(|v| (v, Source::Files)),
                log.map(|v| (v, Source::Log)),
            ]
            .into_iter()
            .flatten()
            .find(|(version, _)| base(version) == dll);
            Some(refined.unwrap_or((dll, Source::Files)))
        }
        None => install_file
            .map(|version| (version, Source::Files))
            .or_else(|| log.map(|version| (version, Source::Log)))
            .or_else(|| variables.map(|version| (version, Source::Variables))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    const CALOPTREYX_EGG: &str = "export DOTNET_ROOT=/home/container/.dotnet DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1 DOTNET_CLI_TELEMETRY_OPTOUT=1; exec ./VintagestoryServer --dataPath /home/container/data {{EXTRA_ARGS}}";
    const PELICAN_EGG: &str = "./VintagestoryServer --dataPath ./data --port={{SERVER_PORT}} --maxclients={{MAX_CLIENTS}} {{OPTIONS}}";

    #[test]
    fn data_path_from_known_eggs() {
        assert_eq!(
            data_path(CALOPTREYX_EGG, &vars(&[("EXTRA_ARGS", "")])),
            ("data".into(), Source::Startup)
        );
        assert_eq!(
            data_path(
                PELICAN_EGG,
                &vars(&[("SERVER_PORT", "42420"), ("OPTIONS", "")])
            ),
            ("data".into(), Source::Startup)
        );
    }

    #[test]
    fn data_path_variants() {
        let none = vars(&[]);
        assert_eq!(
            data_path(
                "dotnet VintagestoryServer.dll --dataPath=\"/home/container/my data\"",
                &none
            )
            .0,
            "my data"
        );
        assert_eq!(
            data_path("./VintagestoryServer --DATAPATH '~/vs'", &none).0,
            "vs"
        );
        assert_eq!(
            data_path(
                "./VintagestoryServer --dataPath {{DATA_DIR}}",
                &vars(&[("DATA_DIR", "./server/../srv")])
            )
            .0,
            "srv"
        );
        assert_eq!(data_path("./VintagestoryServer --dataPath .", &none).0, "");
        // without --dataPath the game uses ~/.config/VintagestoryData
        assert_eq!(
            data_path("./VintagestoryServer --port 42420", &none),
            (DEFAULT_DATA_PATH.into(), Source::Default)
        );
        // outside the server's files: fall back rather than guess
        assert_eq!(
            data_path("./VintagestoryServer --dataPath /srv/vs", &none).1,
            Source::Default
        );
        assert_eq!(
            data_path("./VintagestoryServer --dataPath ../..", &none).1,
            Source::Default
        );
        // an unresolved variable is no path
        assert_eq!(
            data_path("./VintagestoryServer --dataPath", &none),
            (DEFAULT_DATA_PATH.into(), Source::Default)
        );
    }

    #[test]
    fn tokenizer_and_substitution() {
        assert_eq!(
            tokenize(r#"a "b c" 'd "e"' f\ g "h\"i";j&&k"#),
            ["a", "b c", "d \"e\"", "f g", "h\"i", "j", "k"]
        );
        assert_eq!(
            substitute(
                "x {{A}} ${B} {{ env.A }} {{C}} ${",
                &vars(&[("A", "1"), ("B", "2")])
            ),
            "x 1 2 1 {{C}} ${"
        );
    }

    #[test]
    fn logs_follow_log_path() {
        let none = vars(&[]);
        assert_eq!(
            main_log(PELICAN_EGG, &none, "data"),
            "data/Logs/server-main.log"
        );
        assert_eq!(
            main_log(
                "./VintagestoryServer --logPath /home/container/logs",
                &none,
                "data"
            ),
            "logs/server-main.log"
        );
    }

    #[test]
    fn mods_dir_from_server_config() {
        let config =
            crate::json::parse(include_str!("../tests/fixtures/serverconfig.json")).unwrap();
        // the default config lists the binaries' Mods first, then <data>/Mods
        assert_eq!(
            mods_dir("data", Some(&config), Source::Startup),
            ("data/Mods".into(), Source::Config)
        );
        let custom = json!({ "modpaths": ["Mods", "/home/container/mods-extra", "/opt/other"] });
        assert_eq!(
            mods_dir("data", Some(&custom), Source::Startup),
            ("mods-extra".into(), Source::Config)
        );
        let outside = json!({ "ModPaths": ["Mods", "/opt/other"] });
        assert_eq!(
            mods_dir("data", Some(&outside), Source::Startup),
            ("data/Mods".into(), Source::Startup)
        );
        assert_eq!(
            mods_dir("", None, Source::Default),
            ("Mods".into(), Source::Default)
        );
        // with the server root as data path, Mods is both folders
        let root = json!({ "ModPaths": ["Mods", "/home/container/Mods"] });
        assert_eq!(
            mods_dir("", Some(&root), Source::Startup),
            ("Mods".into(), Source::Config)
        );
    }

    #[test]
    fn vintage_story_servers_are_recognised() {
        let none: Vec<String> = Vec::new();
        assert_eq!(
            is_vintage_story(
                "Paper",
                "Minecraft",
                "java",
                "java -jar server.jar",
                &["VintagestoryServer.dll".into()]
            ),
            Some(Source::Files)
        );
        assert_eq!(
            is_vintage_story("Custom", "Games", "img", CALOPTREYX_EGG, &none),
            Some(Source::Startup)
        );
        assert_eq!(
            is_vintage_story(
                "Vintage Story",
                "Survival",
                "ghcr.io/x/dotnet",
                "./start.sh",
                &none
            ),
            Some(Source::Egg)
        );
        assert_eq!(
            is_vintage_story(
                "Paper",
                "Minecraft",
                "ghcr.io/x/java_21",
                "java -jar server.jar",
                &["server.properties".into()]
            ),
            None
        );
    }

    #[test]
    fn game_version_from_the_log() {
        let log = include_str!("../tests/fixtures/server-main.log");
        assert_eq!(game_version_from_log(log), Some("1.22.7".into()));
        assert_eq!(
            game_version_from_log(
                "21.11.2026 [Notification] Game Version: v1.23.0-rc.2 (Unstable)"
            ),
            Some("1.23.0-rc.2".into())
        );
        assert_eq!(game_version_from_log("nothing here"), None);
    }

    #[test]
    fn game_version_from_the_api_dll() {
        let mut dll = b"MZ\x90\0 some code ".to_vec();
        let entry = |key: &str, value: &str| -> Vec<u8> {
            let mut bytes: Vec<u8> = key.encode_utf16().flat_map(u16::to_le_bytes).collect();
            bytes.extend([0, 0, 0, 0]);
            bytes.extend(value.encode_utf16().flat_map(u16::to_le_bytes));
            bytes.extend([0, 0]);
            bytes
        };
        dll.extend(entry("FileVersion", "1.22.0"));
        dll.extend(entry("Assembly Version", "1.22.7.0"));
        assert_eq!(assembly_version(&dll), Some("1.22.7".into()));
        assert_eq!(assembly_version(b"no version resource"), None);

        // 1.21.6: the assembly version is a constant
        let mut old = b"MZ\x90\0 some code ".to_vec();
        old.extend(entry("FileVersion", "1.21.0"));
        old.extend(entry("Assembly Version", "1.0.0.0"));
        assert_eq!(assembly_version(&old), None);
    }

    #[test]
    fn strict_versions() {
        assert_eq!(strict_version("v1.22.7"), Some("1.22.7".into()));
        assert_eq!(strict_version("1.22.0-rc.10"), Some("1.22.0-rc.10".into()));
        for bad in [
            "latest",
            "1.22",
            "1.22.7.0",
            "1.22.0-beta.1",
            "1.22.0-rc",
            "",
        ] {
            assert_eq!(strict_version(bad), None, "{bad}");
        }
        assert_eq!(
            game_version_from_variables(&vars(&[
                ("VS_VERSION", "latest"),
                ("RELEASE_VERSION", "1.21.6")
            ])),
            Some("1.21.6".into())
        );
    }

    #[test]
    fn the_binaries_decide_and_others_refine() {
        let some = |v: &str| Some(v.to_string());
        // a log from before an update is ignored
        assert_eq!(
            pick_game_version(None, some("1.21.6"), some("1.22.7"), None),
            Some(("1.22.7".into(), Source::Files))
        );
        // the log adds the pre-release suffix the dll lacks
        assert_eq!(
            pick_game_version(None, some("1.23.0-rc.2"), some("1.23.0"), None),
            Some(("1.23.0-rc.2".into(), Source::Log))
        );
        assert_eq!(
            pick_game_version(
                some("1.23.0-rc.3"),
                some("1.23.0-rc.2"),
                some("1.23.0"),
                None
            ),
            Some(("1.23.0-rc.3".into(), Source::Files))
        );
        // no access to the binaries
        assert_eq!(
            pick_game_version(None, some("1.21.6"), None, some("1.22.0")),
            Some(("1.21.6".into(), Source::Log))
        );
        assert_eq!(
            pick_game_version(None, None, None, some("1.22.0")),
            Some(("1.22.0".into(), Source::Variables))
        );
        assert_eq!(pick_game_version(None, None, None, None), None);
    }
}
