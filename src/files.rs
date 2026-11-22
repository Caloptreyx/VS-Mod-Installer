//! Server files through Wings, as the acting user sees them (subuser ignore rules apply), with
//! the same error mapping as the panel's own routes. Adapted from Caloptreyx/World-Map.
use axum::http::StatusCode;
use shared::{
    ApiError, State,
    jwt::BasePayload,
    models::{node::Node, server::Server},
    response::ApiResponse,
};
use tokio::io::AsyncReadExt;
use wings_api::client::{ApiHttpError, WingsClient};

const LIST_PER_PAGE: u64 = 200;

pub struct Wings {
    state: State,
    node: Node,
    client: WingsClient,
    server: uuid::Uuid,
    ignored: Option<Vec<compact_str::CompactString>>,
}

/// `/` plus the path relative to the server root.
fn absolute(path: &str) -> compact_str::CompactString {
    compact_str::format_compact!("/{path}")
}

fn too_large(path: &str) -> ApiResponse {
    ApiResponse::error(format!("{path} is too large")).with_status(StatusCode::PAYLOAD_TOO_LARGE)
}

impl Wings {
    pub async fn connect(state: &State, server: &Server) -> Result<Self, ApiResponse> {
        let ignored = server.subuser_ignored_files.clone();
        let node = server.node.fetch_cached(&state.database).await?;
        let client = node
            .api_client(&state.database)
            .await?
            .ignoring(ignored.clone().unwrap_or_default());
        Ok(Self {
            state: state.clone(),
            node,
            client,
            server: server.uuid,
            ignored,
        })
    }

    /// Every entry of the directory (up to `max_pages` pages); `None` when it does not exist.
    pub async fn list(
        &self,
        directory: &str,
        max_pages: u64,
    ) -> Result<Option<Vec<wings_api::DirectoryEntry>>, ApiResponse> {
        let mut query = wings_api::servers_server_files_list::get::Query {
            directory: Some(absolute(directory)),
            ignored: self.ignored.clone(),
            per_page: Some(LIST_PER_PAGE),
            ..Default::default()
        };
        let mut entries = Vec::new();
        for page in 1..=max_pages {
            query.page = Some(page);
            let response = match self
                .client
                .get_servers_server_files_list(self.server, &query)
                .await
            {
                Ok(response) => response,
                Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) if page == 1 => return Ok(None),
                Err(err) => return Err(err.into()),
            };
            let received = response.entries.len() as u64;
            entries.extend(response.entries);
            if received < LIST_PER_PAGE || entries.len() as u64 >= response.total {
                break;
            }
            if page == max_pages {
                tracing::warn!(server = %self.server, directory, "listing truncated after {max_pages} pages");
            }
        }
        Ok(Some(entries))
    }

    /// The file's bytes, at most `max_size` of them; `None` when it does not exist (or is not a
    /// file Wings can read, such as a path inside an archive it can't open).
    pub async fn read(&self, path: &str, max_size: u64) -> Result<Option<Vec<u8>>, ApiResponse> {
        let reader = match self
            .client
            .get_servers_server_files_contents(
                self.server,
                &wings_api::servers_server_files_contents::get::Query {
                    file: Some(absolute(path)),
                    max_size: Some(max_size),
                    ignored: self.ignored.clone(),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(reader) => reader,
            Err(ApiHttpError::Http(StatusCode::NOT_FOUND | StatusCode::EXPECTATION_FAILED, _)) => {
                return Ok(None);
            }
            Err(ApiHttpError::Http(StatusCode::PAYLOAD_TOO_LARGE, err)) => {
                return Err(ApiResponse::new_serialized(ApiError::new_wings_value(err))
                    .with_status(StatusCode::PAYLOAD_TOO_LARGE));
            }
            Err(err) => return Err(err.into()),
        };

        let mut data = Vec::new();
        reader
            .take(max_size.saturating_add(1))
            .read_to_end(&mut data)
            .await?;
        if data.len() as u64 > max_size {
            return Err(too_large(path));
        }
        Ok(Some(data))
    }

    /// The raw bytes of a file, at most `max_size` of them, through the node's download
    /// endpoint with a single-use token, like the panel's own file download. Unlike `contents`
    /// it serves archives: Wings refuses those as text and only opens names ending in `.zip`
    /// as folders, so a disabled `.zip.disabled` mod can only be read this way. `None` when it
    /// does not exist.
    pub async fn download(
        &self,
        path: &str,
        max_size: u64,
    ) -> Result<Option<Vec<u8>>, ApiResponse> {
        // owned payload, signed synchronously: the route futures that read mods concurrently
        // must stay `Send`, which a borrowed payload held across `Node::create_jwt` broke
        #[derive(serde::Serialize)]
        struct FileDownloadJwt {
            #[serde(flatten)]
            base: BasePayload,
            file_path: compact_str::CompactString,
            ignored_files: Vec<compact_str::CompactString>,
            server_uuid: uuid::Uuid,
            unique_id: uuid::Uuid,
        }

        let now = chrono::Utc::now().timestamp();
        let payload = FileDownloadJwt {
            base: BasePayload {
                scope: "file-download".into(),
                issuer: "panel".into(),
                subject: None,
                audience: Vec::new(),
                expiration_time: Some(now + 60),
                not_before: None,
                issued_at: Some(now),
                jwt_id: compact_str::format_compact!("{}", uuid::Uuid::new_v4()),
            },
            file_path: absolute(path),
            ignored_files: self.ignored.clone().unwrap_or_default(),
            server_uuid: self.server,
            unique_id: uuid::Uuid::new_v4(),
        };
        let key = self.node.token.decrypt(&self.state.database).await?;
        let token = self
            .state
            .jwt
            .create_custom(key.as_bytes(), &payload)
            .map_err(anyhow::Error::from)?;

        let mut url = self.node.url("/download/file");
        url.query_pairs_mut().append_pair("token", &token);
        let response = self
            .state
            .client
            .get(url)
            .send()
            .await
            .map_err(anyhow::Error::from)?;
        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        let mut response = response.error_for_status().map_err(anyhow::Error::from)?;
        if response
            .content_length()
            .is_some_and(|length| length > max_size)
        {
            return Err(too_large(path));
        }
        let mut data = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(anyhow::Error::from)? {
            data.extend_from_slice(&chunk);
            if data.len() as u64 > max_size {
                return Err(too_large(path));
            }
        }
        Ok(Some(data))
    }

    /// Lines `1..=lines` of a text file (without reading the rest of a long log); `None` when
    /// it can't be read.
    pub async fn head(&self, path: &str, lines: u64, max_size: u64) -> Option<String> {
        match self
            .client
            .get_servers_server_files_lines(
                self.server,
                &wings_api::servers_server_files_lines::get::Query {
                    file: Some(absolute(path)),
                    start_line: Some(1),
                    end_line: Some(lines),
                    max_size: Some(max_size),
                    ignored: self.ignored.clone(),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(response) => Some(String::from(response.content)),
            Err(err) => {
                tracing::debug!(server = %self.server, path, "could not read lines: {err:?}");
                None
            }
        }
    }
}
