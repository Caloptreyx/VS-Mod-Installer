//! Whether the dependencies in a mod's `modinfo.json` are met by the installed mods.
use crate::{
    modinfo::{BUILTIN_MODS, Dependency},
    versions::Version,
};
use serde::Serialize;
use std::collections::HashMap;

/// What is installed of a mod id: its highest version among enabled files, and whether any
/// file of it is enabled at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub version: String,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Installed and new enough.
    Satisfied,
    /// Part of the game (`game`, `survival`, `creative`).
    Builtin,
    /// `game` needs a newer game version than the server runs.
    GameTooOld,
    /// Installed, but only disabled files of it.
    Disabled,
    /// Installed in a lower version than required.
    Outdated,
    Missing,
}

impl State {
    /// Whether installing from the ModDB would help.
    pub fn installable(self) -> bool {
        matches!(self, Self::Missing | Self::Outdated)
    }
}

/// Keeps, per mod id, the highest version of the enabled files (else of the disabled ones).
pub fn index<'a>(
    mods: impl IntoIterator<Item = (&'a str, &'a str, bool)>,
) -> HashMap<String, Installed> {
    let mut installed: HashMap<String, Installed> = HashMap::new();
    for (modid, version, enabled) in mods {
        let candidate = Installed {
            version: version.to_string(),
            enabled,
        };
        match installed.get_mut(modid) {
            None => {
                installed.insert(modid.to_string(), candidate);
            }
            Some(current) => {
                let better = (enabled && !current.enabled)
                    || (enabled == current.enabled
                        && crate::versions::compare(version, &current.version).is_gt());
                if better {
                    *current = candidate;
                }
            }
        }
    }
    installed
}

fn below(version: &str, required: &str) -> bool {
    match (Version::parse(version), Version::parse(required)) {
        (Some(version), Some(required)) => version < required,
        // unknown versions can't be judged; don't nag about them
        _ => false,
    }
}

pub fn check(
    dependency: &Dependency,
    installed: &HashMap<String, Installed>,
    game_version: Option<&str>,
) -> State {
    if BUILTIN_MODS.contains(&dependency.modid.as_str()) {
        let too_old = dependency.modid == "game"
            && !dependency.version.is_empty()
            && game_version.is_some_and(|game| below(game, &dependency.version));
        return if too_old {
            State::GameTooOld
        } else {
            State::Builtin
        };
    }
    match installed.get(&dependency.modid) {
        None => State::Missing,
        Some(found) if !found.enabled => State::Disabled,
        Some(found)
            if !dependency.version.is_empty() && below(&found.version, &dependency.version) =>
        {
            State::Outdated
        }
        Some(_) => State::Satisfied,
    }
}

/// Mod ids of the dependencies that need attention (missing, outdated or disabled).
pub fn unmet(dependencies: &[Dependency], installed: &HashMap<String, Installed>) -> Vec<String> {
    dependencies
        .iter()
        .filter(|dependency| {
            matches!(
                check(dependency, installed, None),
                State::Missing | State::Outdated | State::Disabled
            )
        })
        .map(|dependency| dependency.modid.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dep(modid: &str, version: &str) -> Dependency {
        Dependency {
            modid: modid.into(),
            version: version.into(),
        }
    }

    fn installed() -> HashMap<String, Installed> {
        index([
            ("carryonlib", "1.0.0", true),
            ("carryonlib", "0.9.0", true),
            ("betterruinslib", "2.0.0", false),
            ("vsimgui", "1.1.0", false),
            ("vsimgui", "1.0.0", true),
        ])
    }

    #[test]
    fn the_index_prefers_enabled_and_newer_files() {
        let installed = installed();
        assert_eq!(installed["carryonlib"].version, "1.0.0");
        assert!(!installed["betterruinslib"].enabled);
        assert_eq!(
            installed["vsimgui"],
            Installed {
                version: "1.0.0".into(),
                enabled: true
            }
        );
    }

    #[test]
    fn dependency_states() {
        let installed = installed();
        let game = Some("1.22.7");
        assert_eq!(
            check(&dep("game", "1.22.0"), &installed, game),
            State::Builtin
        );
        assert_eq!(
            check(&dep("game", "1.23.0"), &installed, game),
            State::GameTooOld
        );
        assert_eq!(
            check(&dep("game", "1.23.0"), &installed, None),
            State::Builtin
        );
        assert_eq!(
            check(&dep("survival", ""), &installed, game),
            State::Builtin
        );
        assert_eq!(
            check(&dep("carryonlib", "1.0.0-rc.1"), &installed, game),
            State::Satisfied
        );
        assert_eq!(
            check(&dep("carryonlib", "1.2.0"), &installed, game),
            State::Outdated
        );
        assert_eq!(
            check(&dep("carryonlib", ""), &installed, game),
            State::Satisfied
        );
        assert_eq!(
            check(&dep("betterruinslib", ""), &installed, game),
            State::Disabled
        );
        assert_eq!(
            check(&dep("primitivesurvival", "3.0.0"), &installed, game),
            State::Missing
        );
        assert!(State::Missing.installable() && State::Outdated.installable());
        assert!(!State::Disabled.installable());
    }

    #[test]
    fn unmet_lists_what_needs_attention() {
        let deps = [
            dep("game", "1.22.0"),
            dep("carryonlib", "1.0.0"),
            dep("betterruinslib", ""),
            dep("primitivesurvival", ""),
        ];
        assert_eq!(
            unmet(&deps, &installed()),
            ["betterruinslib", "primitivesurvival"]
        );
    }
}
