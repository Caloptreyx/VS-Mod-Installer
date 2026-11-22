//! The server API, mounted at `/api/client/servers/{server}/vs-mod-installer`. The panel
//! resolves the server and the user's permissions before these handlers run. Everything here
//! only reads: installs, updates, renames and deletions go through the panel's own file
//! endpoints from the page, so they keep the panel's checks and activity log.
use crate::{
    cache::Memo,
    deps,
    detect::{self, Source, join},
    files::Wings,
    json,
    moddb::{self, ModDetail, ModSummary, ReleaseView, Sort},
    modinfo::{self, ModInfo},
    settings,
    versions::{self, Version},
    zip,
};
use axum::{extract::Query, http::StatusCode, routing::get};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    GetState, State,
    models::{
        server::{GetServer, Server},
        server_variable::ServerVariable,
        user::{GetPermissionManager, GetUser, PermissionManager},
    },
    response::{ApiResponse, ApiResponseResult},
};
use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

const ROOT_MAX_PAGES: u64 = 10;
const MODS_MAX_PAGES: u64 = 50;
const SMALL_FILE: u64 = 1 << 20;
/// `VintagestoryAPI.dll` is about 2 MB.
const DLL_MAX: u64 = 16 << 20;
const LOG_LINES: u64 = 300;
const READ_CONCURRENCY: usize = 6;
const DISABLED_SUFFIX: &str = ".disabled";
const API_DLL: &str = "VintagestoryAPI.dll";

/// Detection results per server and user (subusers may see different files).
static ENVIRONMENTS: LazyLock<Memo<(Uuid, Uuid, bool), Arc<Environment>>> =
    LazyLock::new(|| Memo::new(Duration::from_secs(20), 4096));
/// The version in a `VintagestoryAPI.dll`, by server, size and modification time.
static DLL_VERSIONS: LazyLock<Memo<(Uuid, u64, i64), Option<String>>> =
    LazyLock::new(|| Memo::new(Duration::from_secs(24 * 60 * 60), 4096));
/// Parsed `modinfo.json` per mod file, by server, path, size and modification time.
type InfoKey = (Uuid, String, u64, i64);
static INFOS: LazyLock<Memo<InfoKey, Result<ModInfo, &'static str>>> =
    LazyLock::new(|| Memo::new(Duration::from_secs(24 * 60 * 60), 50_000));

fn bad_request(message: impl AsRef<str>) -> ApiResponse {
    ApiResponse::error(message).with_status(StatusCode::BAD_REQUEST)
}

fn upstream(err: anyhow::Error) -> ApiResponse {
    tracing::warn!("ModDB request failed: {err:#}");
    ApiResponse::error(format!(
        "the Vintage Story ModDB could not be reached: {err}"
    ))
    .with_status(StatusCode::BAD_GATEWAY)
}

// ---------------------------------------------------------------------------------------------
// Detection.

#[derive(Clone, Debug, Serialize)]
pub struct Environment {
    /// Whether this is a Vintage Story server (or the admin allowed its egg).
    pub supported: bool,
    pub detected_by: Option<Source>,
    pub data_path: String,
    pub data_path_source: Source,
    /// Where mods are installed, relative to the server root.
    pub mods_dir: String,
    pub mods_dir_source: Source,
    pub mods_dir_exists: bool,
    pub game_version: Option<String>,
    pub game_version_source: Option<Source>,
    /// Without `files.read-content` mod manifests, the config and the game version can't be
    /// read.
    pub can_read_content: bool,
}

async fn variables(state: &State, server: &Server) -> Vec<(String, String)> {
    match ServerVariable::all_by_server_uuid_egg_uuid(&state.database, server.uuid, server.egg.uuid)
        .await
    {
        Ok(variables) => variables
            .into_iter()
            .filter(|variable| !variable.variable.secret)
            .map(|variable| (variable.variable.env_variable.to_string(), variable.value))
            .collect(),
        Err(err) => {
            tracing::warn!(server = %server.uuid, "failed to load server variables: {err:?}");
            Vec::new()
        }
    }
}

fn parent_and_name(path: &str) -> (&str, &str) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", path),
    }
}

async fn dll_version(
    wings: &Wings,
    server: Uuid,
    entry: &wings_api::DirectoryEntry,
) -> Option<String> {
    let key = (server, entry.size, entry.modified.timestamp());
    if let Some(cached) = DLL_VERSIONS.get(&key) {
        return cached;
    }
    let version = match wings.read(API_DLL, DLL_MAX).await {
        Ok(Some(bytes)) => detect::assembly_version(&bytes),
        _ => None,
    };
    DLL_VERSIONS.insert(key, version.clone());
    version
}

async fn environment(
    state: &State,
    permissions: &PermissionManager,
    server: &mut Server,
    user: Uuid,
) -> Result<Arc<Environment>, ApiResponse> {
    permissions.has_server_permission("files.read")?;
    let can_read_content = permissions
        .has_server_permission("files.read-content")
        .is_ok();
    let key = (server.uuid, user, can_read_content);
    if let Some(environment) = ENVIRONMENTS.get(&key) {
        return Ok(environment);
    }

    let settings = settings::load(state).await?;
    let variables = variables(state, server).await;
    let (data_path, data_path_source) = detect::data_path(&server.startup, &variables);
    let config_path = join(&data_path, "serverconfig.json");
    let log_path = detect::main_log(&server.startup, &variables, &data_path);
    let read_config = can_read_content && !server.is_ignored(&config_path, false);
    let read_log = can_read_content && !server.is_ignored(&log_path, false);
    let read_install_file =
        can_read_content && !server.is_ignored(detect::INSTALL_VERSION_FILE, false);
    let read_dll = can_read_content && !server.is_ignored(API_DLL, false);

    let wings = Wings::connect(state, server).await?;
    let root = wings.list("", ROOT_MAX_PAGES).await?.unwrap_or_default();
    let root_names: Vec<String> = root.iter().map(|entry| entry.name.to_string()).collect();
    let detected = detect::is_vintage_story(
        &server.egg.name,
        &server.nest.name,
        &server.image,
        &server.startup,
        &root_names,
    );
    let (supported, detected_by) = match settings.egg_listed(server.egg.uuid) {
        Some(listed) => (listed, listed.then_some(Source::Settings)),
        None => (detected.is_some(), detected),
    };

    let config = if read_config {
        match wings.read(&config_path, SMALL_FILE).await {
            Ok(Some(bytes)) => json::parse(&String::from_utf8_lossy(&bytes)).ok(),
            _ => None,
        }
    } else {
        None
    };
    let (mods_dir, mods_dir_source) =
        detect::mods_dir(&data_path, config.as_ref(), data_path_source);
    let (parent, name) = parent_and_name(&mods_dir);
    let mods_dir_exists = if parent.is_empty() {
        root.iter()
            .any(|entry| entry.directory && entry.name == name)
    } else {
        matches!(
            wings.list(parent, 5).await,
            Ok(Some(entries)) if entries.iter().any(|entry| entry.directory && entry.name == name)
        )
    };

    let has_install_file = root
        .iter()
        .any(|entry| entry.file && entry.name == detect::INSTALL_VERSION_FILE);
    let install_file = if has_install_file && read_install_file {
        match wings.read(detect::INSTALL_VERSION_FILE, 1024).await {
            Ok(Some(bytes)) => detect::strict_version(&String::from_utf8_lossy(&bytes)),
            _ => None,
        }
    } else {
        None
    };
    let log = match read_log {
        true => wings
            .head(&log_path, LOG_LINES, SMALL_FILE)
            .await
            .and_then(|log| detect::game_version_from_log(&log)),
        false => None,
    };
    let dll = match root
        .iter()
        .find(|entry| entry.file && entry.name == API_DLL)
    {
        Some(entry) if read_dll => dll_version(&wings, server.uuid, entry).await,
        _ => None,
    };
    let game = detect::pick_game_version(
        install_file,
        log,
        dll,
        detect::game_version_from_variables(&variables),
    );

    let environment = Arc::new(Environment {
        supported,
        detected_by,
        data_path,
        data_path_source,
        mods_dir,
        mods_dir_source,
        mods_dir_exists,
        game_version: game.as_ref().map(|(version, _)| version.clone()),
        game_version_source: game.map(|(_, source)| source),
        can_read_content,
    });
    ENVIRONMENTS.insert(key, environment.clone());
    Ok(environment)
}

fn require_supported(environment: &Environment) -> Result<(), ApiResponse> {
    if environment.supported {
        Ok(())
    } else {
        Err(ApiResponse::error("this is not a Vintage Story server")
            .with_status(StatusCode::CONFLICT))
    }
}

/// A game version from the page (strict) or the detected one.
fn game_version(
    requested: Option<&str>,
    environment: &Environment,
) -> Result<Option<String>, ApiResponse> {
    match requested.map(str::trim).filter(|text| !text.is_empty()) {
        Some(text) => detect::strict_version(text)
            .map(Some)
            .ok_or_else(|| bad_request("invalid game version")),
        None => Ok(environment.game_version.clone()),
    }
}

async fn get_environment(
    state: GetState,
    permissions: GetPermissionManager,
    user: GetUser,
    mut server: GetServer,
) -> ApiResponseResult {
    let environment = environment(&state, &permissions, &mut server, user.uuid).await?;
    ApiResponse::new_serialized(json!({ "environment": &*environment })).ok()
}

// ---------------------------------------------------------------------------------------------
// ModDB.

#[derive(Deserialize)]
struct SearchParams {
    #[serde(default)]
    text: String,
    tag: Option<String>,
    game_version: Option<String>,
    #[serde(default)]
    include_client: bool,
    sort: Option<Sort>,
    page: Option<usize>,
    per_page: Option<usize>,
}

async fn search(
    state: GetState,
    permissions: GetPermissionManager,
    user: GetUser,
    mut server: GetServer,
    Query(params): Query<SearchParams>,
) -> ApiResponseResult {
    let environment = environment(&state, &permissions, &mut server, user.uuid).await?;
    require_supported(&environment)?;
    if params.text.len() > 200 {
        return Err(bad_request("the search text is too long"));
    }
    let catalog = moddb::catalog(&state).await.map_err(upstream)?;
    let ids = match params
        .game_version
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        Some(text) => {
            let version =
                detect::strict_version(text).ok_or_else(|| bad_request("invalid game version"))?;
            Some(
                moddb::game_version_ids(&state, &version)
                    .await
                    .map_err(upstream)?,
            )
        }
        None => None,
    };
    let query = moddb::SearchQuery {
        text: params.text,
        tag: params.tag.filter(|tag| !tag.trim().is_empty()),
        include_client: params.include_client,
        sort: params.sort.unwrap_or(Sort::Relevance),
        page: params.page.unwrap_or(1),
        per_page: params.per_page.unwrap_or(20),
    };
    let (total, mods) = moddb::search(&catalog, ids.as_deref(), &query);
    ApiResponse::new_serialized(json!({
        "mods": mods,
        "total": total,
        "page": query.page.max(1),
        "per_page": query.per_page.clamp(1, moddb::MAX_PER_PAGE),
    }))
    .ok()
}

async fn filters(
    state: GetState,
    permissions: GetPermissionManager,
    user: GetUser,
    mut server: GetServer,
) -> ApiResponseResult {
    let environment = environment(&state, &permissions, &mut server, user.uuid).await?;
    require_supported(&environment)?;
    let catalog = moddb::catalog(&state).await.map_err(upstream)?;
    let game_versions = moddb::game_versions(&state).await.map_err(upstream)?;
    ApiResponse::new_serialized(json!({
        "tags": catalog.tags,
        "game_versions": &*game_versions,
    }))
    .ok()
}

#[derive(Deserialize)]
struct DetailParams {
    id: String,
    game_version: Option<String>,
}

/// The fields of a ModDB page the installed list and the dependency dialog show.
#[derive(Clone, Debug, Serialize)]
struct ModRef {
    modid: i64,
    assetid: i64,
    name: String,
    summary: Option<String>,
    logo: Option<String>,
    urlalias: Option<String>,
    side: String,
}

impl From<&ModSummary> for ModRef {
    fn from(summary: &ModSummary) -> Self {
        Self {
            modid: summary.modid,
            assetid: summary.assetid,
            name: summary.name.clone(),
            summary: Some(summary.summary.clone()).filter(|text| !text.is_empty()),
            logo: summary.logo.clone(),
            urlalias: summary.urlalias.clone(),
            side: summary.side.clone(),
        }
    }
}

impl From<&ModDetail> for ModRef {
    fn from(detail: &ModDetail) -> Self {
        Self {
            modid: detail.modid,
            assetid: detail.assetid,
            name: detail.name.clone(),
            summary: None,
            logo: detail.logo.clone(),
            urlalias: detail.urlalias.clone(),
            side: detail.side.clone(),
        }
    }
}

fn release_views<'a>(detail: &'a ModDetail, game: Option<&Version>) -> Vec<ReleaseView<'a>> {
    detail
        .releases
        .iter()
        .map(|release| ReleaseView::new(release, game))
        .collect()
}

async fn mod_detail(
    state: GetState,
    permissions: GetPermissionManager,
    user: GetUser,
    mut server: GetServer,
    Query(params): Query<DetailParams>,
) -> ApiResponseResult {
    let environment = environment(&state, &permissions, &mut server, user.uuid).await?;
    require_supported(&environment)?;
    if !moddb::valid_id(&params.id) {
        return Err(bad_request("invalid mod id"));
    }
    let game = game_version(params.game_version.as_deref(), &environment)?;
    let Some(detail) = moddb::detail(&state, &params.id).await.map_err(upstream)? else {
        return Err(
            ApiResponse::error("the ModDB has no such mod").with_status(StatusCode::NOT_FOUND)
        );
    };
    let parsed = game.as_deref().and_then(Version::parse);
    let recommended =
        versions::recommend(&detail.releases, game.as_deref()).map(|release| release.release_id);
    ApiResponse::new_serialized(json!({
        "mod": &*detail,
        "releases": release_views(&detail, parsed.as_ref()),
        "recommended_release": recommended,
        "game_version": game,
    }))
    .ok()
}

// ---------------------------------------------------------------------------------------------
// Installed mods.

#[derive(Clone, Debug, Serialize)]
struct InstalledMod {
    file_name: String,
    enabled: bool,
    /// `zip`, `dll`, `cs` or `folder`.
    kind: &'static str,
    size: u64,
    modified: String,
    info: Option<ModInfo>,
    /// Why `info` is missing: `permission`, `code_mod`, `no_modinfo`, `invalid_modinfo`,
    /// `invalid_archive`, `too_large` or `unreadable`.
    info_error: Option<&'static str>,
    moddb: Option<ModRef>,
    /// Dependencies that are missing, outdated or only installed disabled.
    unmet_dependencies: Vec<String>,
}

/// The kind of a mod file and whether it is enabled; `None` for files the game doesn't load.
/// Renaming a file to `<name>.disabled` disables it: the game only loads `.zip`, `.dll` and
/// `.cs` files and folders. Folders can't be disabled that way.
fn classify(name: &str, directory: bool) -> Option<(&'static str, bool)> {
    if name.starts_with('.') {
        return None;
    }
    if directory {
        return Some(("folder", true));
    }
    if detect::BUILTIN_MOD_FILES.contains(&name) {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    let (base, enabled) = match lower.strip_suffix(DISABLED_SUFFIX) {
        Some(base) => (base, false),
        None => (lower.as_str(), true),
    };
    let kind = if base.ends_with(".zip") {
        "zip"
    } else if base.ends_with(".dll") {
        "dll"
    } else if base.ends_with(".cs") {
        "cs"
    } else {
        return None;
    };
    Some((kind, enabled))
}

fn parse_info(bytes: &[u8]) -> Result<ModInfo, &'static str> {
    modinfo::parse(bytes).map_err(|_| "invalid_modinfo")
}

async fn read_info(
    wings: &Wings,
    path: &str,
    kind: &str,
    enabled: bool,
    size: u64,
    max_archive: u64,
) -> Result<ModInfo, &'static str> {
    match kind {
        "folder" => match wings.read(&join(path, "modinfo.json"), SMALL_FILE).await {
            Ok(Some(bytes)) => parse_info(&bytes),
            Ok(None) => Err("no_modinfo"),
            Err(_) => Err("unreadable"),
        },
        "zip" => {
            // Wings opens `.zip` files like folders; that reads only the manifest (exact name)
            if enabled
                && path.ends_with(".zip")
                && let Ok(Some(bytes)) = wings.read(&join(path, "modinfo.json"), SMALL_FILE).await
            {
                return parse_info(&bytes);
            }
            // disabled mods, `.ZIP` names and manifests in another case: the whole archive
            if size > max_archive {
                return Err("too_large");
            }
            match wings.download(path, max_archive).await {
                Ok(Some(bytes)) => match zip::read_entry(&bytes, "modinfo.json", SMALL_FILE) {
                    Ok(Some(manifest)) => parse_info(&manifest),
                    Ok(None) => Err("no_modinfo"),
                    Err(zip::ZipError::TooLarge) => Err("too_large"),
                    Err(_) => Err("invalid_archive"),
                },
                Ok(None) | Err(_) => Err("unreadable"),
            }
        }
        _ => Err("code_mod"),
    }
}

struct Installed {
    mods: Vec<InstalledMod>,
    exists: bool,
    catalog_error: Option<String>,
}

async fn installed(
    state: &State,
    server: &mut Server,
    environment: &Environment,
) -> Result<Installed, ApiResponse> {
    if server.is_ignored(&environment.mods_dir, true) {
        return Ok(Installed {
            mods: Vec::new(),
            exists: false,
            catalog_error: None,
        });
    }
    let settings = settings::load(state).await?;
    let max_archive = u64::from(settings.max_archive_mb) << 20;
    let wings = Wings::connect(state, server).await?;
    let Some(entries) = wings.list(&environment.mods_dir, MODS_MAX_PAGES).await? else {
        return Ok(Installed {
            mods: Vec::new(),
            exists: false,
            catalog_error: None,
        });
    };

    let files: Vec<(
        String,
        &'static str,
        bool,
        u64,
        chrono::DateTime<chrono::Utc>,
    )> = entries
        .into_iter()
        .filter(|entry| entry.file || entry.directory)
        .filter_map(|entry| {
            let (kind, enabled) = classify(&entry.name, entry.directory)?;
            Some((
                entry.name.to_string(),
                kind,
                enabled,
                entry.size,
                entry.modified.to_utc(),
            ))
        })
        .collect();

    let server_uuid = server.uuid;
    let can_read = environment.can_read_content;
    let wings = &wings;
    let files_ref = &files;
    // indexes, not tuples: a lifetime in the closure's argument (even `&'static str`) makes the
    // handler future fail the `Send` check axum needs
    let infos: Vec<Result<ModInfo, &'static str>> = stream::iter(0..files.len())
        .map(|index| async move {
            if !can_read {
                return Err("permission");
            }
            let (name, kind, enabled, size, modified) = &files_ref[index];
            let path = join(&environment.mods_dir, name);
            let key = (server_uuid, path.clone(), *size, modified.timestamp());
            if let Some(cached) = INFOS.get(&key) {
                return cached;
            }
            let info = read_info(wings, &path, kind, *enabled, *size, max_archive).await;
            if info != Err("unreadable") {
                INFOS.insert(key, info.clone());
            }
            info
        })
        .buffered(READ_CONCURRENCY)
        .collect()
        .await;

    let (catalog, catalog_error) = match moddb::catalog(state).await {
        Ok(catalog) => (Some(catalog), None),
        Err(err) => {
            tracing::warn!("ModDB list unavailable: {err:#}");
            (None, Some(err.to_string()))
        }
    };

    let index = deps::index(infos.iter().zip(&files).filter_map(|(info, file)| {
        info.as_ref()
            .ok()
            .map(|info| (info.modid.as_str(), info.version.as_str(), file.2))
    }));

    let mut mods: Vec<InstalledMod> = files
        .iter()
        .zip(infos.iter())
        .map(|((name, kind, enabled, size, modified), result)| {
            let info = result.as_ref().ok().cloned();
            InstalledMod {
                file_name: name.clone(),
                enabled: *enabled,
                kind: *kind,
                size: *size,
                modified: modified.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                moddb: info
                    .as_ref()
                    .zip(catalog.as_ref())
                    .and_then(|(info, catalog)| catalog.by_modidstr(&info.modid))
                    .map(ModRef::from),
                unmet_dependencies: info
                    .as_ref()
                    .map(|info| deps::unmet(&info.dependencies, &index))
                    .unwrap_or_default(),
                info_error: result.as_ref().err().copied(),
                info,
            }
        })
        .collect();
    mods.sort_by_cached_key(|installed| {
        let name = installed
            .info
            .as_ref()
            .map(|info| info.name.clone())
            .unwrap_or_else(|| installed.file_name.clone());
        (name.to_lowercase(), installed.file_name.clone())
    });

    Ok(Installed {
        mods,
        exists: true,
        catalog_error,
    })
}

async fn installed_mods(
    state: GetState,
    permissions: GetPermissionManager,
    user: GetUser,
    mut server: GetServer,
) -> ApiResponseResult {
    let environment = environment(&state, &permissions, &mut server, user.uuid).await?;
    require_supported(&environment)?;
    let installed = installed(&state, &mut server, &environment).await?;
    ApiResponse::new_serialized(json!({
        "mods_dir": environment.mods_dir,
        "exists": installed.exists,
        "mods": installed.mods,
        "catalog_error": installed.catalog_error,
    }))
    .ok()
}

#[derive(Deserialize)]
struct GameVersionParams {
    game_version: Option<String>,
}

/// For every installed mod found on the ModDB: the release to update to, when the release
/// picked for the game version is newer than the installed one. One `/api/updates` request
/// finds the mods with any newer release; only those are looked up in full.
async fn updates(
    state: GetState,
    permissions: GetPermissionManager,
    user: GetUser,
    mut server: GetServer,
    Query(params): Query<GameVersionParams>,
) -> ApiResponseResult {
    let environment = environment(&state, &permissions, &mut server, user.uuid).await?;
    require_supported(&environment)?;
    let game = game_version(params.game_version.as_deref(), &environment)?;
    let parsed_game = game.as_deref().and_then(Version::parse);
    let installed = installed(&state, &mut server, &environment).await?;

    let candidates: Vec<(&InstalledMod, &ModInfo, &ModRef)> = installed
        .mods
        .iter()
        .filter_map(|installed| {
            Some((
                installed,
                installed.info.as_ref()?,
                installed.moddb.as_ref()?,
            ))
        })
        .collect();
    let query: Vec<(String, String)> = candidates
        .iter()
        .map(|(_, info, _)| (info.modid.clone(), info.version.clone()))
        .collect();
    let newer = moddb::updates(&state, &query).await.map_err(upstream)?;

    let mut found = Vec::new();
    for (installed, info, moddb_ref) in candidates {
        if !newer.contains_key(&info.modid) {
            continue;
        }
        let Some(detail) = moddb::detail(&state, &moddb_ref.modid.to_string())
            .await
            .map_err(upstream)?
        else {
            continue;
        };
        let Some(release) = versions::recommend(&detail.releases, game.as_deref()) else {
            continue;
        };
        let newer_than_installed = match (
            Version::parse(&release.mod_version),
            Version::parse(&info.version),
        ) {
            (Some(candidate), Some(current)) => candidate > current,
            _ => false,
        };
        if newer_than_installed {
            found.push(json!({
                "file_name": installed.file_name,
                "modid": moddb_ref.modid,
                "release": ReleaseView::new(release, parsed_game.as_ref()),
            }));
        }
    }
    ApiResponse::new_serialized(json!({ "updates": found, "game_version": game })).ok()
}

#[derive(Deserialize)]
struct DependencyParams {
    file: String,
    game_version: Option<String>,
}

/// The dependencies of one installed mod, each with its state and, when it is missing or
/// outdated, the ModDB release that would fix it (one level deep).
async fn dependencies(
    state: GetState,
    permissions: GetPermissionManager,
    user: GetUser,
    mut server: GetServer,
    Query(params): Query<DependencyParams>,
) -> ApiResponseResult {
    let environment = environment(&state, &permissions, &mut server, user.uuid).await?;
    require_supported(&environment)?;
    if params.file.is_empty() || params.file.contains(['/', '\\']) || params.file.starts_with('.') {
        return Err(bad_request("invalid file name"));
    }
    let game = game_version(params.game_version.as_deref(), &environment)?;
    let parsed_game = game.as_deref().and_then(Version::parse);
    let installed = installed(&state, &mut server, &environment).await?;
    let Some(target) = installed
        .mods
        .iter()
        .find(|installed| installed.file_name == params.file)
    else {
        return Err(
            ApiResponse::error("the mod file was not found").with_status(StatusCode::NOT_FOUND)
        );
    };
    let Some(info) = &target.info else {
        return ApiResponse::new_serialized(json!({
            "file_name": target.file_name,
            "info": null,
            "info_error": target.info_error,
            "dependencies": [],
        }))
        .ok();
    };

    let index = deps::index(installed.mods.iter().filter_map(|installed| {
        installed.info.as_ref().map(|info| {
            (
                info.modid.as_str(),
                info.version.as_str(),
                installed.enabled,
            )
        })
    }));

    let mut views = Vec::new();
    for dependency in &info.dependencies {
        let dependency_state = deps::check(dependency, &index, game.as_deref());
        let mut view = json!({
            "modid": dependency.modid,
            "required_version": dependency.version,
            "state": dependency_state,
            "installed_version": index.get(&dependency.modid).map(|found| found.version.clone()),
            "moddb": null,
            "release": null,
            "problem": null,
        });
        if dependency_state.installable() {
            let detail = if moddb::valid_id(&dependency.modid) {
                moddb::detail(&state, &dependency.modid)
                    .await
                    .map_err(upstream)?
            } else {
                None
            };
            match detail {
                None => view["problem"] = json!("not_on_moddb"),
                Some(detail) => {
                    view["moddb"] = json!(ModRef::from(&*detail));
                    let release = versions::recommend(&detail.releases, game.as_deref());
                    let new_enough = release.is_some_and(|release| {
                        dependency.version.is_empty()
                            || versions::compare(&release.mod_version, &dependency.version).is_ge()
                    });
                    match release {
                        Some(release) if new_enough => {
                            view["release"] =
                                json!(ReleaseView::new(release, parsed_game.as_ref()));
                        }
                        Some(_) => view["problem"] = json!("no_new_enough_release"),
                        None => view["problem"] = json!("no_release"),
                    }
                }
            }
        }
        views.push(view);
    }

    ApiResponse::new_serialized(json!({
        "file_name": target.file_name,
        "info": info,
        "info_error": null,
        "dependencies": views,
        "game_version": game,
    }))
    .ok()
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .route("/environment", get(get_environment))
        .route("/moddb/search", get(search))
        .route("/moddb/filters", get(filters))
        .route("/moddb/mod", get(mod_detail))
        .route("/mods", get(installed_mods))
        .route("/mods/updates", get(updates))
        .route("/mods/dependencies", get(dependencies))
        .with_state(state.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_files_are_classified_like_the_game_loads_them() {
        assert_eq!(
            classify("CarryOn-1.22.0_v1.14.3.zip", false),
            Some(("zip", true))
        );
        assert_eq!(
            classify("CarryOn.ZIP.disabled", false),
            Some(("zip", false))
        );
        assert_eq!(classify("tweak.dll", false), Some(("dll", true)));
        assert_eq!(classify("script.cs.disabled", false), Some(("cs", false)));
        assert_eq!(classify("mymod", true), Some(("folder", true)));
        // the game ignores these
        assert_eq!(classify("Creating_Mods.txt", false), None);
        assert_eq!(classify("notes.zip.bak", false), None);
        assert_eq!(classify(".git", true), None);
        assert_eq!(classify("VSSurvivalMod.dll", false), None);
    }

    #[test]
    fn paths_split_into_parent_and_name() {
        assert_eq!(parent_and_name("data/Mods"), ("data", "Mods"));
        assert_eq!(parent_and_name("Mods"), ("", "Mods"));
    }
}
