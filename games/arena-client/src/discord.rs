use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

const DISCORD_CLIENT_ID: &str = "1532447557668704336";
const RECONNECT_INTERVAL: Duration = Duration::from_secs(15);

const NOMINAL_MAX_PLAYERS: i32 = 16;

pub struct DiscordPresence {
    client: Option<DiscordIpcClient>,
    next_connect_attempt: Instant,
    start_time_ms: i64,
    last_details: String,
    last_state: String,
}

impl DiscordPresence {
    pub fn new() -> Self {
        let mut presence = Self {
            client: None,
            next_connect_attempt: Instant::now(),
            start_time_ms: unix_time_ms(),
            last_details: String::new(),
            last_state: String::new(),
        };
        presence.try_connect();
        presence
    }

    fn try_connect(&mut self) {
        if self.client.is_some() || Instant::now() < self.next_connect_attempt {
            return;
        }
        self.next_connect_attempt = Instant::now() + RECONNECT_INTERVAL;
        let mut client = DiscordIpcClient::new(DISCORD_CLIENT_ID);
        match client.connect() {
            Ok(()) => {
                log::info!("Discord Rich Presence connected");
                self.client = Some(client);
            }
            Err(err) => log::debug!("Discord Rich Presence not available yet: {err}"),
        }
    }

    fn set(
        &mut self,
        details: &str,
        state: &str,
        small_image: &str,
        small_text: &str,
        party: Option<(u32, i32)>,
    ) {
        self.try_connect();
        let Some(client) = &mut self.client else {
            return;
        };

        if self.last_details == details && self.last_state == state {
            return;
        }

        let mut activity = activity::Activity::new()
            .details(details)
            .state(state)
            .assets(
                activity::Assets::new()
                    .large_image("logo_large")
                    .large_text("RustForge Arena")
                    .small_image(small_image)
                    .small_text(small_text),
            )
            .timestamps(activity::Timestamps::new().start(self.start_time_ms));

        if let Some((current, max)) = party {
            activity = activity.party(
                activity::Party::new()
                    .id("arena-lobby")
                    .size([current as i32, max]),
            );
        }

        if client.set_activity(activity).is_err() {
            self.client = None;
        } else {
            self.last_details = details.to_string();
            self.last_state = state.to_string();
        }
    }

    pub fn set_menu(&mut self) {
        self.set(
            "In Main Menu",
            "Not connected",
            "status_menu",
            "Main Menu",
            None,
        );
    }

    pub fn set_connecting(&mut self, address: &str) {
        self.set(
            &format!("Connecting to {address}"),
            "Joining server...",
            "status_menu",
            "Connecting",
            None,
        );
    }

    pub fn set_lobby(&mut self, ready: u32, total: u32) {
        self.set(
            "In the Pregame Lobby",
            &format!("{ready}/{total} ready"),
            "status_lobby",
            "Pregame Lobby",
            Some((total, NOMINAL_MAX_PLAYERS)),
        );
    }

    pub fn set_playing(&mut self, kills: u32, deaths: u32, players_alive: u32, players_total: u32) {
        self.set(
            &format!("{kills} Kills - {deaths} Deaths"),
            &format!("{players_alive}/{players_total} players alive"),
            "status_playing",
            "In a Match",
            Some((players_total, NOMINAL_MAX_PLAYERS)),
        );
    }

    pub fn set_ending(&mut self, won: bool, kills: u32) {
        self.set(
            if won { "Victory!" } else { "Match Ended" },
            &format!("{kills} kills this match"),
            "status_victory",
            "Match Results",
            None,
        );
    }
}

impl Default for DiscordPresence {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        if let Some(client) = &mut self.client {
            let _ = client.close();
        }
    }
}

fn unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
