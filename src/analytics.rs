//! Anonymous WebHatchery game telemetry for native and WebGL builds.
//!
//! The client deliberately accepts a small vocabulary of decision-useful events.
//! It batches delivery, retries failed batches, and counts only time that the game
//! reports as active. Games remain responsible for deciding what "active" means.

use crate::net::{HttpClient, Pending};
use crate::persistence::{load_string_key, save_string_key};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const INSTALLATION_KEY: &str = "analytics_installation_id";
const SCHEMA_VERSION: u8 = 1;
const MAX_QUEUE: usize = 100;
const MAX_BATCH: usize = 25;
const REQUEST_TIMEOUT_SECONDS: f32 = 8.0;
const RETRY_SECONDS: f32 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventName {
    SessionStarted,
    SessionHeartbeat,
    Milestone,
    DemoCompleted,
    FullGameClicked,
    GameCompleted,
    SessionEnded,
}

#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    pub endpoint: String,
    pub write_key: String,
    pub game: String,
    pub version: String,
    pub platform: String,
    pub source: String,
    pub heartbeat_seconds: f32,
    pub flush_seconds: f32,
}

impl AnalyticsConfig {
    pub fn web(
        endpoint: impl Into<String>,
        write_key: impl Into<String>,
        game: impl Into<String>,
        version: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            write_key: write_key.into(),
            game: game.into(),
            version: version.into(),
            platform: "web".to_owned(),
            source: source.into(),
            heartbeat_seconds: 60.0,
            flush_seconds: 30.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct QueuedEvent {
    event_id: String,
    installation_id: String,
    session_id: String,
    event: AnalyticsEventName,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    active_seconds: u64,
    session_elapsed_seconds: u64,
}

#[derive(Serialize)]
struct EventBatch<'a> {
    schema_version: u8,
    game: &'a str,
    version: &'a str,
    platform: &'a str,
    source: &'a str,
    events: &'a [QueuedEvent],
}

#[derive(Debug, Deserialize)]
struct IngestEnvelope {
    success: bool,
}

pub struct AnalyticsClient {
    config: AnalyticsConfig,
    http: HttpClient,
    installation_id: String,
    session_id: String,
    queued: VecDeque<QueuedEvent>,
    in_flight_events: Vec<QueuedEvent>,
    in_flight: Option<Pending<IngestEnvelope>>,
    active_seconds: f32,
    elapsed_seconds: f32,
    heartbeat_elapsed: f32,
    flush_elapsed: f32,
    retry_remaining: f32,
    flush_requested: bool,
    ended: bool,
}

impl AnalyticsClient {
    pub fn new(config: AnalyticsConfig) -> Self {
        let installation_id = load_string_key(&config.game, INSTALLATION_KEY)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                let id = random_id();
                let _ = save_string_key(&config.game, INSTALLATION_KEY, &id);
                id
            });
        Self::with_ids(config, installation_id, random_id())
    }

    fn with_ids(config: AnalyticsConfig, installation_id: String, session_id: String) -> Self {
        let mut http = HttpClient::new(&config.endpoint);
        http.set_header("X-WebHatchery-Analytics-Key", config.write_key.clone());
        let mut client = Self {
            config,
            http,
            installation_id,
            session_id,
            queued: VecDeque::new(),
            in_flight_events: Vec::new(),
            in_flight: None,
            active_seconds: 0.0,
            elapsed_seconds: 0.0,
            heartbeat_elapsed: 0.0,
            flush_elapsed: 0.0,
            retry_remaining: 0.0,
            flush_requested: false,
            ended: false,
        };
        client.emit(AnalyticsEventName::SessionStarted, None);
        client
    }

    /// Advance telemetry from the game loop. Pass `true` only while the player
    /// is meaningfully interacting or unpaused in foreground gameplay.
    pub fn update(&mut self, dt: f32, is_active: bool) {
        let dt = dt.clamp(0.0, 1.0);
        self.elapsed_seconds += dt;
        self.flush_elapsed += dt;
        self.retry_remaining = (self.retry_remaining - dt).max(0.0);

        if is_active && !self.ended {
            self.active_seconds += dt;
            self.heartbeat_elapsed += dt;
            if self.heartbeat_elapsed >= self.config.heartbeat_seconds.max(1.0) {
                self.heartbeat_elapsed = 0.0;
                self.emit(AnalyticsEventName::SessionHeartbeat, None);
            }
        }

        self.poll_request(dt);
        let periodic_flush = self.flush_elapsed >= self.config.flush_seconds.max(1.0);
        if self.in_flight.is_none()
            && self.retry_remaining <= 0.0
            && !self.queued.is_empty()
            && (periodic_flush || self.flush_requested)
        {
            self.begin_flush();
        }
    }

    pub fn milestone(&mut self, name: impl Into<String>) {
        self.emit(AnalyticsEventName::Milestone, Some(name.into()));
        self.request_flush();
    }

    pub fn demo_completed(&mut self) {
        self.emit(AnalyticsEventName::DemoCompleted, None);
        self.request_flush();
    }

    pub fn full_game_clicked(&mut self) {
        self.emit(AnalyticsEventName::FullGameClicked, None);
        self.request_flush();
    }

    pub fn game_completed(&mut self) {
        self.emit(AnalyticsEventName::GameCompleted, None);
        self.request_flush();
    }

    pub fn end_session(&mut self) {
        if !self.ended {
            self.ended = true;
            self.emit(AnalyticsEventName::SessionEnded, None);
            self.request_flush();
        }
    }

    pub fn request_flush(&mut self) {
        self.flush_requested = true;
    }

    pub fn queued_count(&self) -> usize {
        self.queued.len() + self.in_flight_events.len()
    }

    fn emit(&mut self, event: AnalyticsEventName, value: Option<String>) {
        if self.queued.len() >= MAX_QUEUE {
            self.queued.pop_front();
        }
        self.queued.push_back(QueuedEvent {
            event_id: random_id(),
            installation_id: self.installation_id.clone(),
            session_id: self.session_id.clone(),
            event,
            value: value.map(|value| value.chars().take(100).collect()),
            active_seconds: self.active_seconds.floor() as u64,
            session_elapsed_seconds: self.elapsed_seconds.floor() as u64,
        });
    }

    fn begin_flush(&mut self) {
        self.in_flight_events = (0..MAX_BATCH)
            .filter_map(|_| self.queued.pop_front())
            .collect();
        let batch = EventBatch {
            schema_version: SCHEMA_VERSION,
            game: &self.config.game,
            version: &self.config.version,
            platform: &self.config.platform,
            source: &self.config.source,
            events: &self.in_flight_events,
        };
        self.in_flight = Some(self.http.post_json("", &batch));
        self.flush_elapsed = 0.0;
        self.flush_requested = false;
    }

    fn poll_request(&mut self, dt: f32) {
        let Some(pending) = self.in_flight.as_mut() else {
            return;
        };
        let Some(result) = pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS) else {
            return;
        };
        self.in_flight = None;
        match result {
            Ok(envelope) if envelope.success => self.in_flight_events.clear(),
            _ => {
                for event in self.in_flight_events.drain(..).rev() {
                    self.queued.push_front(event);
                }
                self.retry_remaining = RETRY_SECONDS;
            }
        }
        if !self.queued.is_empty() {
            self.flush_requested = true;
        }
    }
}

fn random_id() -> String {
    let a = macroquad::rand::rand();
    let b = macroquad::rand::rand();
    let c = macroquad::rand::rand();
    let d = macroquad::rand::rand();
    format!("{a:08x}-{b:08x}-{c:08x}-{d:08x}")
}

#[cfg(test)]
mod tests;
