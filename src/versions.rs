//! Version strings of the game and of mods, and which mod release suits a game version.
//!
//! Vintage Story and the ModDB use `major.minor.patch` with an optional `-dev.N`, `-pre.N` or
//! `-rc.N` suffix; a version with a suffix comes before the same version without one. Mods put
//! looser strings in `modinfo.json` (`v1.2`, `1.0.0-beta`), so parsing is lenient: any number
//! of numeric parts, an optional leading `v` and any suffix.
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct Version {
    parts: Vec<u64>,
    /// `None` for a stable version, else the rank of the suffix kind and its number.
    pre: Option<(u8, u64)>,
}

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text
            .strip_prefix('v')
            .or_else(|| text.strip_prefix('V'))
            .unwrap_or(text);
        // build metadata never orders versions
        let text = text.split('+').next().unwrap_or(text);
        let (core, suffix) = match text.split_once('-') {
            Some((core, suffix)) => (core, Some(suffix)),
            None => (text, None),
        };
        let parts = core
            .split('.')
            .map(|part| part.parse::<u64>().ok())
            .collect::<Option<Vec<_>>>()?;
        if parts.is_empty() || parts.len() > 6 {
            return None;
        }
        let pre = suffix.map(|suffix| {
            let suffix = suffix.to_ascii_lowercase();
            let kind: String = suffix
                .chars()
                .take_while(|ch| ch.is_ascii_alphabetic())
                .collect();
            let number: String = suffix
                .chars()
                .rev()
                .take_while(|ch| ch.is_ascii_digit())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let rank = match kind.as_str() {
                "dev" | "alpha" | "a" => 1,
                "pre" | "beta" | "b" | "preview" => 2,
                "rc" => 3,
                _ => 0,
            };
            (rank, number.parse().unwrap_or(0))
        });
        Some(Self { parts, pre })
    }

    pub fn is_prerelease(&self) -> bool {
        self.pre.is_some()
    }

    /// Major and minor version, which the ModDB calls the "major version" of the game.
    pub fn primary(&self) -> (u64, u64) {
        (
            self.parts.first().copied().unwrap_or(0),
            self.parts.get(1).copied().unwrap_or(0),
        )
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let len = self.parts.len().max(other.parts.len());
        for i in 0..len {
            let a = self.parts.get(i).copied().unwrap_or(0);
            let b = other.parts.get(i).copied().unwrap_or(0);
            match a.cmp(&b) {
                Ordering::Equal => {}
                unequal => return unequal,
            }
        }
        match (self.pre, other.pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(&b),
        }
    }
}

/// Compares two version strings; unparsable ones sort first, then by text.
pub fn compare(a: &str, b: &str) -> Ordering {
    match (Version::parse(a), Version::parse(b)) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => a.cmp(b),
    }
}

/// How well a mod release fits the server's game version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    /// The release lists neither the game version nor its major version.
    Incompatible,
    /// The game version is not known.
    Unknown,
    /// The release lists another patch of the same major version (`1.21.x`).
    SameMajor,
    /// The release lists the exact game version.
    Exact,
}

pub fn compatibility(game_versions: &[String], game: Option<&Version>) -> Compatibility {
    let Some(game) = game else {
        return Compatibility::Unknown;
    };
    let listed: Vec<Version> = game_versions
        .iter()
        .filter_map(|tag| Version::parse(tag))
        .collect();
    if listed.iter().any(|version| version == game) {
        Compatibility::Exact
    } else if listed
        .iter()
        .any(|version| version.primary() == game.primary())
    {
        Compatibility::SameMajor
    } else {
        Compatibility::Incompatible
    }
}

/// What release selection needs to know about a release.
pub trait ReleaseLike {
    fn mod_version(&self) -> &str;
    fn game_versions(&self) -> &[String];
    fn created(&self) -> &str;
    fn installable(&self) -> bool;
}

/// The release to install for `game`: the newest stable release made for exactly that game
/// version, else the newest pre-release for it, then the same two steps for releases made for
/// another patch of the same major version, and last the newest release at all (which the user
/// is warned about). Releases without a file are never chosen.
pub fn recommend<'a, R: ReleaseLike>(releases: &'a [R], game: Option<&str>) -> Option<&'a R> {
    let game = game.and_then(Version::parse);
    let ranked = releases
        .iter()
        .filter(|release| release.installable())
        .map(|release| {
            let version = Version::parse(release.mod_version());
            let fit = match compatibility(release.game_versions(), game.as_ref()) {
                Compatibility::Exact => 3,
                Compatibility::SameMajor => 2,
                Compatibility::Unknown | Compatibility::Incompatible => 1,
            };
            let stable = !version.as_ref().is_some_and(Version::is_prerelease);
            (fit, stable, version, release)
        });
    ranked
        .max_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.cmp(&b.1))
                .then_with(|| match (&a.2, &b.2) {
                    (Some(x), Some(y)) => x.cmp(y),
                    (Some(_), None) => Ordering::Greater,
                    (None, Some(_)) => Ordering::Less,
                    (None, None) => Ordering::Equal,
                })
                .then_with(|| a.3.created().cmp(b.3.created()))
        })
        .map(|(_, _, _, release)| release)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn versions_order_like_the_moddb() {
        assert!(v("1.21.0") > v("1.20.12"));
        assert!(v("1.22.0-rc.1") < v("1.22.0"));
        assert!(v("1.22.0-pre.3") < v("1.22.0-rc.1"));
        assert!(v("1.22.0-dev.9") < v("1.22.0-pre.1"));
        assert!(v("1.22.0-rc.10") > v("1.22.0-rc.9"));
        assert_eq!(v("v1.2"), v("1.2.0"));
        assert_eq!(v("1.2.0+build5"), v("1.2.0"));
        assert!(v("2.0.0-beta") < v("2.0.0"));
        assert!(Version::parse("latest").is_none());
        assert!(Version::parse("").is_none());
        assert_eq!(v("1.21").primary(), (1, 21));
    }

    #[test]
    fn compare_puts_unparsable_versions_first() {
        assert_eq!(compare("1.0.0", "garbage"), Ordering::Greater);
        assert_eq!(compare("1.0.10", "1.0.9"), Ordering::Greater);
    }

    struct R(&'static str, Vec<String>, &'static str, bool);

    fn rel(version: &'static str, tags: &[&str], created: &'static str, installable: bool) -> R {
        R(
            version,
            tags.iter().map(|tag| tag.to_string()).collect(),
            created,
            installable,
        )
    }

    impl ReleaseLike for R {
        fn mod_version(&self) -> &str {
            self.0
        }
        fn game_versions(&self) -> &[String] {
            &self.1
        }
        fn created(&self) -> &str {
            self.2
        }
        fn installable(&self) -> bool {
            self.3
        }
    }

    #[test]
    fn compatibility_levels() {
        let tags = vec!["1.21.0".to_string(), "1.21.1".to_string()];
        assert_eq!(
            compatibility(&tags, Some(&v("1.21.1"))),
            Compatibility::Exact
        );
        assert_eq!(
            compatibility(&tags, Some(&v("1.21.6"))),
            Compatibility::SameMajor
        );
        assert_eq!(
            compatibility(&tags, Some(&v("1.22.0"))),
            Compatibility::Incompatible
        );
        assert_eq!(compatibility(&tags, None), Compatibility::Unknown);
    }

    #[test]
    fn the_newest_stable_release_for_the_exact_game_version_wins() {
        let releases = [
            rel("2.1.0", &["1.22.0", "1.22.1"], "2026-10-01", true),
            rel("2.2.0-rc.1", &["1.21.7"], "2026-10-05", true),
            rel("2.0.1", &["1.21.7"], "2026-09-01", true),
            rel("2.0.0", &["1.21.7"], "2026-08-01", true),
            rel("1.9.0", &["1.20.12"], "2026-01-01", true),
        ];
        assert_eq!(recommend(&releases, Some("1.21.7")).unwrap().0, "2.0.1");
        assert_eq!(recommend(&releases, Some("1.22.1")).unwrap().0, "2.1.0");
        // 1.22.5 is not listed, but 1.22.x is
        assert_eq!(recommend(&releases, Some("1.22.5")).unwrap().0, "2.1.0");
        // nothing for 1.19: the newest stable release overall, with a warning in the UI
        assert_eq!(recommend(&releases, Some("1.19.8")).unwrap().0, "2.1.0");
        assert_eq!(recommend(&releases, None).unwrap().0, "2.1.0");
    }

    #[test]
    fn pre_releases_only_win_when_nothing_stable_fits() {
        let releases = [
            rel("3.0.0-pre.2", &["1.23.0-pre.1"], "2026-11-02", true),
            rel("2.0.0", &["1.22.7"], "2026-10-01", true),
        ];
        assert_eq!(
            recommend(&releases, Some("1.23.0-pre.1")).unwrap().0,
            "3.0.0-pre.2"
        );
        assert_eq!(recommend(&releases, Some("1.22.7")).unwrap().0, "2.0.0");
    }

    #[test]
    fn releases_without_a_file_are_skipped() {
        let releases = [
            rel("2.0.0", &["1.22.7"], "2026-10-01", false),
            rel("1.0.0", &["1.22.7"], "2026-09-01", true),
        ];
        assert_eq!(recommend(&releases, Some("1.22.7")).unwrap().0, "1.0.0");
        let none = [rel("2.0.0", &["1.22.7"], "2026-10-01", false)];
        assert!(recommend(&none, Some("1.22.7")).is_none());
    }
}
