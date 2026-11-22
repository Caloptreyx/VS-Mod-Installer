//! `modinfo.json`, the manifest of content mods and of most code mods, read the way Vintage
//! Story reads it (Newtonsoft.Json, see [`crate::json`]): property names in any case, comments
//! and trailing commas allowed, `modid` derived from `name` when it is missing.
use crate::json;
use serde::Serialize;
use serde_json::Value;

/// Mod ids that are part of the game itself and never installed from the ModDB.
pub const BUILTIN_MODS: [&str; 3] = ["game", "survival", "creative"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Dependency {
    pub modid: String,
    /// The minimum version, or empty for any version (`""` and `"*"` in the file).
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModInfo {
    pub modid: String,
    pub name: String,
    pub version: String,
    /// `content`, `code` or `theme`, lower case.
    pub kind: Option<String>,
    /// `universal`, `client` or `server`, lower case (the game's default is universal).
    pub side: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ModInfoError {
    Json(String),
    NotAnObject,
    /// Neither `modid` nor a `name` it could be derived from.
    NoModId,
}

impl std::fmt::Display for ModInfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(err) => write!(f, "invalid modinfo.json: {err}"),
            Self::NotAnObject => f.write_str("modinfo.json is not an object"),
            Self::NoModId => f.write_str("modinfo.json has neither a modid nor a name"),
        }
    }
}

/// The game's `ModInfo.ToModID`: letters and digits of the name, lower case. A name starting
/// with a digit can't be turned into an id.
pub fn to_modid(name: &str) -> Option<String> {
    if name.starts_with(|ch: char| ch.is_ascii_digit()) {
        return None;
    }
    let id: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|ch| ch.to_ascii_lowercase())
        .collect();
    (!id.is_empty()).then_some(id)
}

fn string(value: &Value, key: &str) -> Option<String> {
    json::get(value, key)
        .and_then(json::scalar_string)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub fn parse(bytes: &[u8]) -> Result<ModInfo, ModInfoError> {
    let text = String::from_utf8_lossy(bytes);
    let value = json::parse(&text).map_err(|err| ModInfoError::Json(err.to_string()))?;
    if !value.is_object() {
        return Err(ModInfoError::NotAnObject);
    }

    let name = string(&value, "name");
    let modid = string(&value, "modid")
        .or_else(|| name.as_deref().and_then(to_modid))
        .ok_or(ModInfoError::NoModId)?
        .to_ascii_lowercase();

    let authors = match json::get(&value, "authors") {
        Some(Value::Array(items)) => items.iter().filter_map(json::scalar_string).collect(),
        Some(single) => json::scalar_string(single).into_iter().collect(),
        None => Vec::new(),
    };

    let dependencies = match json::get(&value, "dependencies") {
        Some(Value::Object(entries)) => entries
            .iter()
            .map(|(modid, version)| Dependency {
                modid: modid.trim().to_ascii_lowercase(),
                version: json::scalar_string(version)
                    .map(|version| version.trim().to_string())
                    .filter(|version| version != "*")
                    .unwrap_or_default(),
            })
            .filter(|dependency| !dependency.modid.is_empty())
            .collect(),
        _ => Vec::new(),
    };

    Ok(ModInfo {
        name: name.unwrap_or_else(|| modid.clone()),
        modid,
        version: string(&value, "version").unwrap_or_default(),
        kind: string(&value, "type").map(|kind| kind.to_ascii_lowercase()),
        side: string(&value, "side")
            .map(|side| side.to_ascii_lowercase())
            .unwrap_or_else(|| "universal".into()),
        description: string(&value, "description"),
        authors,
        dependencies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = include_str!("../tests/fixtures/modinfo/full.json");
    const LENIENT: &str = include_str!("../tests/fixtures/modinfo/lenient.json");
    const NAME_ONLY: &str = include_str!("../tests/fixtures/modinfo/name-only.json");

    #[test]
    fn a_regular_modinfo_parses() {
        let info = parse(FULL.as_bytes()).unwrap();
        assert_eq!(info.modid, "carryon");
        assert_eq!(info.name, "Carry On");
        assert_eq!(info.version, "1.8.0");
        assert_eq!(info.kind.as_deref(), Some("code"));
        assert_eq!(info.side, "universal");
        assert_eq!(info.authors, ["copygirl", "Xytabich"]);
        assert_eq!(
            info.dependencies,
            [
                Dependency {
                    modid: "game".into(),
                    version: "1.21.0".into()
                },
                Dependency {
                    modid: "vsimgui".into(),
                    version: String::new()
                },
            ]
        );
    }

    #[test]
    fn keys_in_any_case_comments_and_trailing_commas_are_accepted() {
        let info = parse(LENIENT.as_bytes()).unwrap();
        assert_eq!(info.modid, "betterruins");
        assert_eq!(info.version, "0.6.5");
        assert_eq!(info.side, "server");
        assert_eq!(info.kind.as_deref(), Some("content"));
        assert_eq!(info.authors, ["NiclAss"]);
        assert_eq!(
            info.dependencies,
            [
                Dependency {
                    modid: "game".into(),
                    version: "1.22.0".into()
                },
                Dependency {
                    modid: "betterruinslib".into(),
                    version: "1.0.2".into()
                },
            ]
        );
    }

    #[test]
    fn the_modid_comes_from_the_name_when_missing() {
        let info = parse(NAME_ONLY.as_bytes()).unwrap();
        assert_eq!(info.modid, "primitivesurvival");
        assert_eq!(info.name, "Primitive Survival");
        assert!(info.dependencies.is_empty());
        assert_eq!(to_modid("1stmod"), None);
        assert_eq!(to_modid("A-B c_9"), Some("abc9".into()));
    }

    #[test]
    fn broken_files_are_errors() {
        assert!(matches!(parse(b"{ name: "), Err(ModInfoError::Json(_))));
        assert_eq!(parse(b"[]"), Err(ModInfoError::NotAnObject));
        assert_eq!(
            parse(b"{\"version\": \"1.0.0\"}"),
            Err(ModInfoError::NoModId)
        );
    }
}
