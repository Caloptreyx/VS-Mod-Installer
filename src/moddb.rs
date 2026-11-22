//! The official Vintage Story ModDB (<https://mods.vintagestory.at>, public API v1, no key).
//!
//! Endpoints used:
//! - `GET /api/mods`: every mod (about 10,000 entries, 4 MB) with name, summary, author, side,
//!   type, logo, tags, counters and the mod ids (`modidstrs`) of its releases. Fetched once and
//!   searched, filtered, sorted and paged here, so typing in the search box never reaches the
//!   ModDB.
//! - `GET /api/mods?gv=<version>`: the mods with a release for one game version (ids only are
//!   kept), for the game version filter.
//! - `GET /api/mod/<id or modid>`: one mod with its HTML description and releases.
//! - `GET /api/gameversions`: every game version.
//! - `GET /api/updates?mods=<modid>@<version>,...`: the newest release of each mod when it is
//!   newer than the given version, to find updates with one request.
//!
//! The API answers `200 OK` with `"statuscode": "404"` in the body for unknown mods.
use crate::{
    cache::Memo,
    versions::{self, Compatibility, ReleaseLike, Version},
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use shared::State;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

pub const BASE_URL: &str = "https://mods.vintagestory.at";
const USER_AGENT: &str = concat!(
    "Calagopus-VS-Mod-Installer/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/Caloptreyx/VS-Mod-Installer)"
);
const TIMEOUT: Duration = Duration::from_secs(30);
const LIST_TTL: Duration = Duration::from_secs(10 * 60);
/// After a failed refresh the old list is served this long before the next attempt.
const RETRY_AFTER: Duration = Duration::from_secs(60);
const DETAIL_TTL: Duration = Duration::from_secs(5 * 60);
const UPDATES_BATCH: usize = 40;
pub const MAX_PER_PAGE: usize = 50;

// ---------------------------------------------------------------------------------------------
// Lenient field readers: the API is PHP, so a field may be null, a number or a string.

fn de_num<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|float| float as i64))
            .unwrap_or(0),
        Value::String(text) => text.trim().parse().unwrap_or(0),
        _ => 0,
    })
}

fn de_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::String(text) => Some(text.trim().to_string()).filter(|text| !text.is_empty()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

fn de_strings<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| match item {
                Value::String(text) => {
                    Some(text.trim().to_string()).filter(|text| !text.is_empty())
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    })
}

fn status_code(value: &Value) -> Option<i64> {
    match value {
        Value::String(text) => text.trim().parse().ok(),
        Value::Number(number) => number.as_i64(),
        _ => None,
    }
}

/// `2026-11-21 14:36:09` (the ModDB's UTC timestamps) as RFC 3339.
fn timestamp(text: Option<String>) -> Option<String> {
    let text = text?;
    chrono::NaiveDateTime::parse_from_str(&text, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|time| {
            time.and_utc()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        })
}

fn side(text: Option<String>) -> String {
    match text.as_deref().map(str::to_ascii_lowercase).as_deref() {
        Some("client") => "client".into(),
        Some("server") => "server".into(),
        _ => "both".into(),
    }
}

// ---------------------------------------------------------------------------------------------
// The mod list.

#[derive(Deserialize)]
struct RawSummary {
    #[serde(default, deserialize_with = "de_num")]
    modid: i64,
    #[serde(default, deserialize_with = "de_num")]
    assetid: i64,
    #[serde(default, deserialize_with = "de_num")]
    downloads: i64,
    #[serde(default, deserialize_with = "de_num")]
    follows: i64,
    #[serde(default, deserialize_with = "de_num")]
    trendingpoints: i64,
    #[serde(default, deserialize_with = "de_num")]
    comments: i64,
    #[serde(default, deserialize_with = "de_text")]
    name: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    summary: Option<String>,
    #[serde(default, deserialize_with = "de_strings")]
    modidstrs: Vec<String>,
    #[serde(default, deserialize_with = "de_text")]
    author: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    urlalias: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    side: Option<String>,
    #[serde(default, rename = "type", deserialize_with = "de_text")]
    kind: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    logo: Option<String>,
    #[serde(default, deserialize_with = "de_strings")]
    tags: Vec<String>,
    #[serde(default, deserialize_with = "de_text")]
    lastreleased: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModSummary {
    pub modid: i64,
    pub assetid: i64,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub urlalias: Option<String>,
    /// `client`, `server` or `both`.
    pub side: String,
    /// `mod`, `externaltool` or `other`.
    pub kind: String,
    pub logo: Option<String>,
    pub tags: Vec<String>,
    /// The mod ids (`modid` in `modinfo.json`) of the mod's releases, lower case.
    pub modidstrs: Vec<String>,
    pub downloads: i64,
    pub follows: i64,
    pub trending: i64,
    pub comments: i64,
    pub last_released: Option<String>,
}

impl From<RawSummary> for ModSummary {
    fn from(raw: RawSummary) -> Self {
        Self {
            modid: raw.modid,
            assetid: raw.assetid,
            name: raw.name.unwrap_or_else(|| format!("#{}", raw.modid)),
            summary: raw.summary.unwrap_or_default(),
            author: raw.author.unwrap_or_default(),
            urlalias: raw.urlalias,
            side: side(raw.side),
            kind: raw
                .kind
                .map(|kind| kind.to_ascii_lowercase())
                .unwrap_or_else(|| "mod".into()),
            logo: raw.logo.filter(|logo| logo.starts_with("https://")),
            tags: raw.tags,
            modidstrs: raw
                .modidstrs
                .into_iter()
                .map(|id| id.to_ascii_lowercase())
                .collect(),
            downloads: raw.downloads,
            follows: raw.follows,
            trending: raw.trendingpoints,
            comments: raw.comments,
            last_released: timestamp(raw.lastreleased),
        }
    }
}

#[derive(Deserialize)]
struct ListBody {
    #[serde(default)]
    statuscode: Value,
    #[serde(default)]
    mods: Vec<RawSummary>,
}

pub fn parse_list(bytes: &[u8]) -> anyhow::Result<Vec<ModSummary>> {
    let body: ListBody = serde_json::from_slice(bytes)?;
    if status_code(&body.statuscode) != Some(200) {
        anyhow::bail!("the ModDB answered with status {}", body.statuscode);
    }
    Ok(body
        .mods
        .into_iter()
        .filter(|raw| raw.modid > 0)
        .map(ModSummary::from)
        .collect())
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct TagCount {
    pub name: String,
    pub count: usize,
}

/// The mod list with an index by mod id and the tags in use.
pub struct Catalog {
    pub mods: Vec<ModSummary>,
    by_modidstr: HashMap<String, usize>,
    pub tags: Vec<TagCount>,
}

impl Catalog {
    pub fn new(mods: Vec<ModSummary>) -> Self {
        let mut by_modidstr: HashMap<String, usize> = HashMap::new();
        let mut tags: HashMap<&str, usize> = HashMap::new();
        for (index, summary) in mods.iter().enumerate() {
            for id in &summary.modidstrs {
                // a mod id claimed by several pages (forks, reuploads): the most downloaded wins
                let keep = by_modidstr
                    .get(id)
                    .is_some_and(|&other| mods[other].downloads >= summary.downloads);
                if !keep {
                    by_modidstr.insert(id.clone(), index);
                }
            }
            for tag in &summary.tags {
                *tags.entry(tag.as_str()).or_default() += 1;
            }
        }
        let mut tags: Vec<TagCount> = tags
            .into_iter()
            .map(|(name, count)| TagCount {
                name: name.to_string(),
                count,
            })
            .collect();
        tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        Self {
            mods,
            by_modidstr,
            tags,
        }
    }

    /// The ModDB page of a mod id from `modinfo.json`.
    pub fn by_modidstr(&self, modid: &str) -> Option<&ModSummary> {
        self.by_modidstr
            .get(&modid.to_ascii_lowercase())
            .map(|&index| &self.mods[index])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    Relevance,
    Downloads,
    Follows,
    Trending,
    Updated,
    Newest,
}

pub struct SearchQuery {
    pub text: String,
    pub tag: Option<String>,
    /// Show mods that only run on the client (useless on a server).
    pub include_client: bool,
    pub sort: Sort,
    pub page: usize,
    pub per_page: usize,
}

/// Lower-cased search terms; every one has to match.
fn terms(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(str::to_lowercase)
        .filter(|term| !term.is_empty())
        .collect()
}

fn matches_terms(summary: &ModSummary, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = format!(
        "{}\n{}\n{}\n{}",
        summary.name,
        summary.summary,
        summary.author,
        summary.modidstrs.join(" ")
    )
    .to_lowercase();
    terms.iter().all(|term| haystack.contains(term.as_str()))
}

/// 0 is best: the name equals the search, starts with it, contains every term, else the match
/// was in the summary, author or mod id.
fn relevance(summary: &ModSummary, text: &str, terms: &[String]) -> u8 {
    let name = summary.name.to_lowercase();
    let text = text.trim().to_lowercase();
    if name == text || summary.modidstrs.contains(&text) {
        0
    } else if name.starts_with(&text) {
        1
    } else if terms.iter().all(|term| name.contains(term.as_str())) {
        2
    } else {
        3
    }
}

/// Filters, sorts and pages the catalog. `game_version_ids` limits the result to mods with a
/// release for the chosen game version. External tools (desktop programs) are never listed.
pub fn search<'a>(
    catalog: &'a Catalog,
    game_version_ids: Option<&HashSet<i64>>,
    query: &SearchQuery,
) -> (usize, Vec<&'a ModSummary>) {
    let terms = terms(&query.text);
    let tag = query.tag.as_deref().map(str::to_lowercase);
    let mut hits: Vec<&ModSummary> = catalog
        .mods
        .iter()
        .filter(|summary| summary.kind != "externaltool")
        .filter(|summary| query.include_client || summary.side != "client")
        .filter(|summary| {
            tag.as_ref().is_none_or(|tag| {
                summary
                    .tags
                    .iter()
                    .any(|candidate| candidate.to_lowercase() == *tag)
            })
        })
        .filter(|summary| game_version_ids.is_none_or(|ids| ids.contains(&summary.modid)))
        .filter(|summary| matches_terms(summary, &terms))
        .collect();

    let by_downloads = |a: &&ModSummary, b: &&ModSummary| b.downloads.cmp(&a.downloads);
    match query.sort {
        Sort::Relevance if !terms.is_empty() => hits.sort_by(|a, b| {
            relevance(a, &query.text, &terms)
                .cmp(&relevance(b, &query.text, &terms))
                .then_with(|| by_downloads(a, b))
        }),
        Sort::Relevance | Sort::Downloads => hits.sort_by(by_downloads),
        Sort::Follows => {
            hits.sort_by(|a, b| b.follows.cmp(&a.follows).then_with(|| by_downloads(a, b)))
        }
        Sort::Trending => {
            hits.sort_by(|a, b| b.trending.cmp(&a.trending).then_with(|| by_downloads(a, b)))
        }
        Sort::Updated => hits.sort_by(|a, b| b.last_released.cmp(&a.last_released)),
        // mod ids grow with every new page
        Sort::Newest => hits.sort_by(|a, b| b.modid.cmp(&a.modid)),
    }

    let total = hits.len();
    let per_page = query.per_page.clamp(1, MAX_PER_PAGE);
    let page = query.page.max(1);
    let start = (page - 1).saturating_mul(per_page).min(total);
    let end = start.saturating_add(per_page).min(total);
    (total, hits[start..end].to_vec())
}

#[derive(Deserialize)]
struct IdOnly {
    #[serde(default, deserialize_with = "de_num")]
    modid: i64,
}

#[derive(Deserialize)]
struct IdListBody {
    #[serde(default)]
    statuscode: Value,
    #[serde(default)]
    mods: Vec<IdOnly>,
}

pub fn parse_ids(bytes: &[u8]) -> anyhow::Result<HashSet<i64>> {
    let body: IdListBody = serde_json::from_slice(bytes)?;
    if status_code(&body.statuscode) != Some(200) {
        anyhow::bail!("the ModDB answered with status {}", body.statuscode);
    }
    Ok(body.mods.into_iter().map(|entry| entry.modid).collect())
}

// ---------------------------------------------------------------------------------------------
// One mod.

#[derive(Deserialize)]
struct RawRelease {
    #[serde(default, deserialize_with = "de_num")]
    releaseid: i64,
    #[serde(default, deserialize_with = "de_text")]
    mainfile: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    filename: Option<String>,
    #[serde(default, deserialize_with = "de_num")]
    downloads: i64,
    #[serde(default, deserialize_with = "de_strings")]
    tags: Vec<String>,
    #[serde(default, deserialize_with = "de_text")]
    modidstr: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    modversion: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    created: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    changelog: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Release {
    pub release_id: i64,
    /// The download URL on the ModDB's CDN; empty when the release has no file.
    pub mainfile: String,
    /// The original file name, which the installed file keeps.
    pub filename: String,
    pub downloads: i64,
    /// Compatible game versions, newest first.
    pub game_versions: Vec<String>,
    pub modidstr: String,
    pub mod_version: String,
    pub created: Option<String>,
    /// HTML.
    pub changelog: Option<String>,
}

/// A file name that is safe to create in the mods folder and that the game loads.
pub fn is_mod_file_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('.')
        && !name.contains(['/', '\\', '\0'])
        && (lower.ends_with(".zip") || lower.ends_with(".dll") || lower.ends_with(".cs"))
}

impl From<RawRelease> for Release {
    fn from(raw: RawRelease) -> Self {
        let mut game_versions = raw.tags;
        game_versions.sort_by(|a, b| versions::compare(b, a));
        game_versions.dedup();
        Self {
            release_id: raw.releaseid,
            mainfile: raw.mainfile.unwrap_or_default(),
            filename: raw.filename.unwrap_or_default(),
            downloads: raw.downloads,
            game_versions,
            modidstr: raw.modidstr.unwrap_or_default().to_ascii_lowercase(),
            mod_version: raw.modversion.unwrap_or_default(),
            created: timestamp(raw.created),
            changelog: raw.changelog,
        }
    }
}

impl ReleaseLike for Release {
    fn mod_version(&self) -> &str {
        &self.mod_version
    }

    fn game_versions(&self) -> &[String] {
        &self.game_versions
    }

    fn created(&self) -> &str {
        self.created.as_deref().unwrap_or_default()
    }

    fn installable(&self) -> bool {
        self.mainfile.starts_with("https://") && is_mod_file_name(&self.filename)
    }
}

/// A release as sent to the page: how it fits the server's game version, and whether it can be
/// installed at all.
#[derive(Serialize)]
pub struct ReleaseView<'a> {
    #[serde(flatten)]
    pub release: &'a Release,
    pub compatibility: Compatibility,
    pub prerelease: bool,
    pub installable: bool,
}

impl<'a> ReleaseView<'a> {
    pub fn new(release: &'a Release, game: Option<&Version>) -> Self {
        Self {
            compatibility: versions::compatibility(&release.game_versions, game),
            prerelease: Version::parse(&release.mod_version).is_some_and(|v| v.is_prerelease()),
            installable: release.installable(),
            release,
        }
    }
}

#[derive(Deserialize)]
struct RawScreenshot {
    #[serde(default, deserialize_with = "de_text")]
    mainfile: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    thumbnailfilename: Option<String>,
}

#[derive(Deserialize)]
struct RawDetail {
    #[serde(default, deserialize_with = "de_num")]
    modid: i64,
    #[serde(default, deserialize_with = "de_num")]
    assetid: i64,
    #[serde(default, deserialize_with = "de_text")]
    name: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    text: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    author: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    urlalias: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    logofile: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    logofiledb: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    homepageurl: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    sourcecodeurl: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    trailervideourl: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    issuetrackerurl: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    wikiurl: Option<String>,
    #[serde(default, deserialize_with = "de_num")]
    downloads: i64,
    #[serde(default, deserialize_with = "de_num")]
    follows: i64,
    #[serde(default, deserialize_with = "de_num")]
    comments: i64,
    #[serde(default, deserialize_with = "de_text")]
    side: Option<String>,
    #[serde(default, rename = "type", deserialize_with = "de_text")]
    kind: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    created: Option<String>,
    #[serde(default, deserialize_with = "de_text")]
    lastreleased: Option<String>,
    #[serde(default, deserialize_with = "de_strings")]
    tags: Vec<String>,
    #[serde(default)]
    releases: Vec<RawRelease>,
    #[serde(default)]
    screenshots: Vec<RawScreenshot>,
}

#[derive(Debug, Serialize)]
pub struct Links {
    pub homepage: Option<String>,
    pub source: Option<String>,
    pub issues: Option<String>,
    pub wiki: Option<String>,
    pub trailer: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Screenshot {
    pub url: String,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ModDetail {
    pub modid: i64,
    pub assetid: i64,
    pub name: String,
    /// HTML written by the mod author; the page sanitizes it.
    pub text: String,
    pub author: String,
    pub urlalias: Option<String>,
    pub logo: Option<String>,
    pub links: Links,
    pub downloads: i64,
    pub follows: i64,
    pub comments: i64,
    pub side: String,
    pub kind: String,
    pub created: Option<String>,
    pub last_released: Option<String>,
    pub tags: Vec<String>,
    pub screenshots: Vec<Screenshot>,
    /// Newest first, as the ModDB lists them.
    #[serde(skip)]
    pub releases: Vec<Release>,
}

fn https(url: Option<String>) -> Option<String> {
    url.filter(|url| url.starts_with("https://") || url.starts_with("http://"))
}

impl From<RawDetail> for ModDetail {
    fn from(raw: RawDetail) -> Self {
        Self {
            modid: raw.modid,
            assetid: raw.assetid,
            name: raw.name.unwrap_or_else(|| format!("#{}", raw.modid)),
            text: raw.text.unwrap_or_default(),
            author: raw.author.unwrap_or_default(),
            urlalias: raw.urlalias,
            logo: https(raw.logofile.or(raw.logofiledb)),
            links: Links {
                homepage: https(raw.homepageurl),
                source: https(raw.sourcecodeurl),
                issues: https(raw.issuetrackerurl),
                wiki: https(raw.wikiurl),
                trailer: https(raw.trailervideourl),
            },
            downloads: raw.downloads,
            follows: raw.follows,
            comments: raw.comments,
            side: side(raw.side),
            kind: raw
                .kind
                .map(|kind| kind.to_ascii_lowercase())
                .unwrap_or_else(|| "mod".into()),
            created: timestamp(raw.created),
            last_released: timestamp(raw.lastreleased),
            tags: raw.tags,
            screenshots: raw
                .screenshots
                .into_iter()
                .filter_map(|shot| {
                    Some(Screenshot {
                        url: https(shot.mainfile)?,
                        thumbnail: https(shot.thumbnailfilename),
                    })
                })
                .collect(),
            releases: raw.releases.into_iter().map(Release::from).collect(),
        }
    }
}

#[derive(Deserialize)]
struct DetailBody {
    #[serde(default)]
    statuscode: Value,
    #[serde(default, rename = "mod")]
    found: Option<RawDetail>,
}

/// `None` when the ModDB doesn't know the mod.
pub fn parse_detail(bytes: &[u8]) -> anyhow::Result<Option<ModDetail>> {
    let body: DetailBody = serde_json::from_slice(bytes)?;
    match (status_code(&body.statuscode), body.found) {
        (Some(404), _) => Ok(None),
        (Some(200), Some(found)) if found.modid > 0 => Ok(Some(found.into())),
        (status, _) => anyhow::bail!("the ModDB answered with status {status:?}"),
    }
}

#[derive(Deserialize)]
struct UpdatesBody {
    #[serde(default)]
    statuscode: Value,
    #[serde(default)]
    updates: HashMap<String, RawRelease>,
}

/// The newest release of each listed mod id that has a newer one.
pub fn parse_updates(bytes: &[u8]) -> anyhow::Result<HashMap<String, Release>> {
    let body: UpdatesBody = serde_json::from_slice(bytes)?;
    if status_code(&body.statuscode) != Some(200) {
        anyhow::bail!("the ModDB answered with status {}", body.statuscode);
    }
    Ok(body
        .updates
        .into_iter()
        .map(|(modid, release)| (modid.to_ascii_lowercase(), release.into()))
        .collect())
}

#[derive(Deserialize)]
struct RawGameVersion {
    #[serde(default, deserialize_with = "de_text")]
    name: Option<String>,
}

#[derive(Deserialize)]
struct GameVersionsBody {
    #[serde(default)]
    statuscode: Value,
    #[serde(default)]
    gameversions: Vec<RawGameVersion>,
}

/// Every game version, newest first.
pub fn parse_game_versions(bytes: &[u8]) -> anyhow::Result<Vec<String>> {
    let body: GameVersionsBody = serde_json::from_slice(bytes)?;
    if status_code(&body.statuscode) != Some(200) {
        anyhow::bail!("the ModDB answered with status {}", body.statuscode);
    }
    let mut names: Vec<String> = body
        .gameversions
        .into_iter()
        .filter_map(|version| version.name)
        .collect();
    names.sort_by(|a, b| versions::compare(b, a));
    names.dedup();
    Ok(names)
}

/// A mod id as used in ModDB URLs and `modinfo.json` (letters and digits), or a numeric id.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

/// Whether `modid@version` can go into the updates query as it is (no escaping needed).
fn updates_entry(modid: &str, version: &str) -> Option<String> {
    let version_ok = !version.is_empty()
        && version.len() <= 32
        && version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-');
    (valid_id(modid) && version_ok).then(|| format!("{}@{version}", modid.to_ascii_lowercase()))
}

// ---------------------------------------------------------------------------------------------
// Fetching, with caches shared by every server on the panel.

async fn fetch(state: &State, path_and_query: &str) -> anyhow::Result<Vec<u8>> {
    let response = state
        .client
        .get(format!("{BASE_URL}{path_and_query}"))
        .header("User-Agent", USER_AGENT)
        .timeout(TIMEOUT)
        .send()
        .await?
        .error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

type Slot<T> = tokio::sync::Mutex<Option<(Instant, Arc<T>)>>;

static CATALOG: LazyLock<Slot<Catalog>> = LazyLock::new(|| tokio::sync::Mutex::new(None));
static GAME_VERSIONS: LazyLock<Slot<Vec<String>>> = LazyLock::new(|| tokio::sync::Mutex::new(None));
static GAME_VERSION_IDS: LazyLock<Memo<String, Arc<HashSet<i64>>>> =
    LazyLock::new(|| Memo::new(LIST_TTL, 64));
static DETAILS: LazyLock<Memo<String, Option<Arc<ModDetail>>>> =
    LazyLock::new(|| Memo::new(DETAIL_TTL, 4096));
static UPDATES: LazyLock<Memo<String, Arc<HashMap<String, Release>>>> =
    LazyLock::new(|| Memo::new(DETAIL_TTL, 1024));

/// One value fetched at a time (concurrent callers wait for the same fetch), kept for `ttl`; a
/// failed refresh serves the old value for a minute before trying again.
async fn single_flight<T, F, Fut>(slot: &Slot<T>, ttl: Duration, load: F) -> anyhow::Result<Arc<T>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut guard = slot.lock().await;
    if let Some((at, value)) = guard.as_ref()
        && at.elapsed() < ttl
    {
        return Ok(value.clone());
    }
    match load().await {
        Ok(value) => {
            let value = Arc::new(value);
            *guard = Some((Instant::now(), value.clone()));
            Ok(value)
        }
        Err(err) => match guard.take() {
            Some((_, stale)) => {
                tracing::warn!("ModDB request failed, serving the previous answer: {err:#}");
                let retry_at = Instant::now()
                    .checked_sub(ttl.saturating_sub(RETRY_AFTER))
                    .unwrap_or_else(Instant::now);
                *guard = Some((retry_at, stale.clone()));
                Ok(stale)
            }
            None => Err(err),
        },
    }
}

pub async fn catalog(state: &State) -> anyhow::Result<Arc<Catalog>> {
    single_flight(&CATALOG, LIST_TTL, || async {
        Ok(Catalog::new(parse_list(&fetch(state, "/api/mods").await?)?))
    })
    .await
}

pub async fn game_versions(state: &State) -> anyhow::Result<Arc<Vec<String>>> {
    single_flight(&GAME_VERSIONS, Duration::from_secs(60 * 60), || async {
        parse_game_versions(&fetch(state, "/api/gameversions").await?)
    })
    .await
}

/// Ids of the mods with a release for `version` (a strict game version).
pub async fn game_version_ids(state: &State, version: &str) -> anyhow::Result<Arc<HashSet<i64>>> {
    if let Some(ids) = GAME_VERSION_IDS.get(&version.to_string()) {
        return Ok(ids);
    }
    let ids = Arc::new(parse_ids(
        &fetch(state, &format!("/api/mods?gv={version}")).await?,
    )?);
    GAME_VERSION_IDS.insert(version.to_string(), ids.clone());
    Ok(ids)
}

/// A mod by numeric id or mod id (`valid_id` must hold); `None` when the ModDB doesn't know it.
pub async fn detail(state: &State, id: &str) -> anyhow::Result<Option<Arc<ModDetail>>> {
    let key = id.to_ascii_lowercase();
    if let Some(cached) = DETAILS.get(&key) {
        return Ok(cached);
    }
    let found = parse_detail(&fetch(state, &format!("/api/mod/{key}")).await?)?.map(Arc::new);
    DETAILS.insert(key, found.clone());
    if let Some(found) = &found {
        DETAILS.insert(found.modid.to_string(), Some(found.clone()));
    }
    Ok(found)
}

/// The newest release of each `(modid, installed version)` that has a newer one, keyed by mod
/// id. Entries that can't be put in the query are skipped.
pub async fn updates(
    state: &State,
    installed: &[(String, String)],
) -> anyhow::Result<HashMap<String, Release>> {
    let mut entries: Vec<String> = installed
        .iter()
        .filter_map(|(modid, version)| updates_entry(modid, version))
        .collect();
    entries.sort();
    entries.dedup();
    let mut found = HashMap::new();
    for batch in entries.chunks(UPDATES_BATCH) {
        let query = batch.join(",");
        let result = match UPDATES.get(&query) {
            Some(result) => result,
            None => {
                let result = Arc::new(parse_updates(
                    &fetch(state, &format!("/api/updates?mods={query}")).await?,
                )?);
                UPDATES.insert(query, result.clone());
                result
            }
        };
        found.extend(
            result
                .iter()
                .map(|(modid, release)| (modid.clone(), release.clone())),
        );
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = include_str!("../tests/fixtures/moddb/mods.json");
    const DETAIL: &str = include_str!("../tests/fixtures/moddb/mod-carryon.json");
    const MISSING: &str = include_str!("../tests/fixtures/moddb/mod-missing.json");
    const UPDATES_JSON: &str = include_str!("../tests/fixtures/moddb/updates.json");
    const GAME_VERSIONS_JSON: &str = include_str!("../tests/fixtures/moddb/gameversions.json");

    fn catalog() -> Catalog {
        Catalog::new(parse_list(LIST.as_bytes()).unwrap())
    }

    fn query(text: &str) -> SearchQuery {
        SearchQuery {
            text: text.into(),
            tag: None,
            include_client: false,
            sort: Sort::Relevance,
            page: 1,
            per_page: 20,
        }
    }

    #[test]
    fn the_mod_list_parses() {
        let mods = parse_list(LIST.as_bytes()).unwrap();
        assert_eq!(mods.len(), 7);
        let ruins = mods.iter().find(|m| m.modid == 792).unwrap();
        assert_eq!(ruins.name, "BetterRuins");
        assert_eq!(ruins.modidstrs, ["betterruins"]);
        assert_eq!(ruins.side, "both");
        assert_eq!(ruins.kind, "mod");
        assert!(
            ruins
                .logo
                .as_deref()
                .unwrap()
                .starts_with("https://moddbcdn.vintagestory.at/")
        );
        assert!(ruins.last_released.as_deref().unwrap().ends_with('Z'));
        // `"tags": [""]` and `"logo": null` on a mod without them
        let bare = mods.iter().find(|m| m.modid == 13688).unwrap();
        assert!(bare.tags.is_empty());
        assert_eq!(bare.logo, None);
        assert!(parse_list(br#"{"statuscode":"500"}"#).is_err());
    }

    #[test]
    fn search_hides_client_mods_and_tools_by_default() {
        let catalog = catalog();
        let (total, hits) = search(&catalog, None, &query(""));
        assert_eq!(total, 5);
        assert!(
            hits.iter()
                .all(|m| m.side != "client" && m.kind != "externaltool")
        );
        // sorted by downloads without a search text
        assert!(
            hits.windows(2)
                .all(|pair| pair[0].downloads >= pair[1].downloads)
        );
        let with_client = SearchQuery {
            include_client: true,
            ..query("")
        };
        assert_eq!(search(&catalog, None, &with_client).0, 6);
    }

    #[test]
    fn search_text_tags_versions_and_paging() {
        let catalog = catalog();
        let (_, hits) = search(&catalog, None, &query("carry"));
        assert_eq!(hits[0].name, "Carry On");
        // terms match the mod id too
        let (_, hits) = search(&catalog, None, &query("zindustrialstoryfixes"));
        assert_eq!(hits.len(), 1);
        let tagged = SearchQuery {
            tag: Some("worldgen".into()),
            ..query("")
        };
        assert_eq!(search(&catalog, None, &tagged).1[0].modid, 792);
        let ids: HashSet<i64> = [890].into();
        assert_eq!(search(&catalog, Some(&ids), &query("")).0, 1);
        let paged = SearchQuery {
            per_page: 2,
            page: 3,
            sort: Sort::Newest,
            ..query("")
        };
        let (total, hits) = search(&catalog, None, &paged);
        assert_eq!((total, hits.len()), (5, 1));
        let past_the_end = SearchQuery { page: 99, ..paged };
        assert!(search(&catalog, None, &past_the_end).1.is_empty());
    }

    #[test]
    fn the_catalog_finds_mods_by_modinfo_id_and_counts_tags() {
        let catalog = catalog();
        assert_eq!(catalog.by_modidstr("CarryOn").unwrap().modid, 890);
        assert_eq!(
            catalog.by_modidstr("industrialstoryfixes").unwrap().modid,
            13548
        );
        assert!(catalog.by_modidstr("nope").is_none());
        assert!(
            catalog
                .tags
                .iter()
                .any(|tag| tag.name == "QoL" && tag.count >= 2)
        );
        assert!(
            catalog
                .tags
                .windows(2)
                .all(|pair| pair[0].count >= pair[1].count)
        );
    }

    #[test]
    fn a_mod_parses_with_its_releases() {
        let detail = parse_detail(DETAIL.as_bytes()).unwrap().unwrap();
        assert_eq!(detail.modid, 890);
        assert_eq!(detail.name, "Carry On");
        // empty links are dropped
        assert_eq!(detail.links.homepage, None);
        assert!(detail.links.issues.is_some());
        assert_eq!(detail.releases.len(), 6);
        let first = &detail.releases[0];
        assert_eq!(first.mod_version, "2.0.0-rc.1");
        assert_eq!(first.modidstr, "carryon");
        assert_eq!(first.game_versions[0], "1.22.7");
        assert!(first.installable());
        assert!(parse_detail(MISSING.as_bytes()).unwrap().is_none());
    }

    #[test]
    fn the_right_release_of_a_real_mod_is_picked() {
        let detail = parse_detail(DETAIL.as_bytes()).unwrap().unwrap();
        let pick = |game: &str| {
            versions::recommend(&detail.releases, Some(game))
                .unwrap()
                .mod_version
                .clone()
        };
        // a stable release beats a newer release candidate for the same game version
        assert_eq!(pick("1.22.7"), "1.14.3");
        assert_eq!(pick("1.21.7"), "1.13.0");
        assert_eq!(pick("1.21.3"), "1.13.0");
        assert_eq!(pick("1.19.8"), "1.7.4");
        let view = ReleaseView::new(&detail.releases[0], Version::parse("1.22.7").as_ref());
        assert_eq!(view.compatibility, Compatibility::Exact);
        assert!(view.prerelease);
    }

    #[test]
    fn updates_and_game_versions_parse() {
        let updates = parse_updates(UPDATES_JSON.as_bytes()).unwrap();
        assert_eq!(updates["carryon"].mod_version, "2.0.0-rc.1");
        let versions = parse_game_versions(GAME_VERSIONS_JSON.as_bytes()).unwrap();
        assert_eq!(versions[0], "1.23.0-pre.1");
        assert_eq!(versions[1], "1.22.7");
        assert!(
            versions.iter().position(|v| v == "1.22.0").unwrap()
                < versions.iter().position(|v| v == "1.22.0-rc.10").unwrap()
        );
    }

    #[test]
    fn ids_and_file_names_are_checked() {
        assert!(valid_id("carryon") && valid_id("890"));
        assert!(!valid_id("../etc") && !valid_id("") && !valid_id("a b"));
        assert_eq!(
            updates_entry("CarryOn", "1.14.2"),
            Some("carryon@1.14.2".into())
        );
        assert_eq!(updates_entry("carryon", "1.0+x"), None);
        assert!(is_mod_file_name("CarryOn-1.22.0_v1.14.3.zip"));
        assert!(
            !is_mod_file_name("../x.zip")
                && !is_mod_file_name(".hidden.zip")
                && !is_mod_file_name("readme.txt")
        );
    }
}
