//! Real WebSocket client for the Kairo `/api/v1/events` stream, with sequence
//! and gap detection (`snapshot.required` / `event.gap` semantics).

use anyhow::{Context as _, Result};
use http_client::http::Request;
use serde::Deserialize;

pub type EventStream = async_tungstenite::WebSocketStream<smol::net::TcpStream>;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KairoEvent {
    #[serde(default)]
    pub sequence: u64,
    #[serde(rename = "type", default)]
    pub event_type: String,
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default)]
    pub time: String,
}

/// Connect to the agent's event stream. The agent authenticates the WS upgrade
/// with the `kairo-secret-v1` subprotocol and an optional secret header.
pub async fn connect(
    base_url: &str,
    secret: Option<&str>,
    workspace_id: &str,
) -> Result<EventStream> {
    let authority = base_url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .context("invalid agent base url")?;
    let tcp = smol::net::TcpStream::connect(authority)
        .await
        .with_context(|| format!("connecting to Kairo agent at {authority}"))?;

    let uri = format!("/api/v1/events?workspaceId={workspace_id}");
    let mut builder = Request::builder()
        .uri(uri)
        .header("Host", authority)
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("Sec-WebSocket-Protocol", "kairo-secret-v1");
    if let Some(secret) = secret {
        builder = builder.header("X-Kairo-Secret", secret);
    }
    let request = builder.body(()).context("building WS handshake request")?;
    let (stream, _) = async_tungstenite::client_async(request, tcp)
        .await
        .context("WebSocket upgrade failed")?;
    Ok(stream)
}

/// Track event sequence numbers and report gaps that require a snapshot.
#[derive(Debug, Default)]
pub struct EventTracker {
    pub last_sequence: u64,
    pub gap_count: u32,
    pub events: Vec<KairoEvent>,
}

impl EventTracker {
    pub fn ingest(&mut self, event: KairoEvent) -> bool {
        let had_gap = self.last_sequence != 0 && event.sequence > self.last_sequence + 1;
        if had_gap {
            self.gap_count += 1;
        }
        if event.sequence > self.last_sequence {
            self.last_sequence = event.sequence;
        }
        self.events.push(event);
        if self.events.len() > 1000 {
            self.events.remove(0);
        }
        had_gap
    }
}
