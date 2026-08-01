use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub last_address: String,
    pub last_name: String,
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_address: "127.0.0.1:50051".to_string(),
            last_name: "PLAYER".to_string(),
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 0.9,
        }
    }
}

impl AppConfig {
    fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default()
            .join("arena_config.txt")
    }

    pub fn load() -> Self {
        let mut config = Self::default();
        let Ok(contents) = std::fs::read_to_string(Self::path()) else {
            return config;
        };
        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            match key {
                "last_address" if !value.is_empty() => config.last_address = value.to_string(),
                "last_name" if !value.is_empty() => config.last_name = value.to_string(),
                "master_volume" => {
                    if let Ok(v) = value.parse() {
                        config.master_volume = v;
                    }
                }
                "music_volume" => {
                    if let Ok(v) = value.parse() {
                        config.music_volume = v;
                    }
                }
                "sfx_volume" => {
                    if let Ok(v) = value.parse() {
                        config.sfx_volume = v;
                    }
                }
                _ => {}
            }
        }
        config
    }

    pub fn save(&self) {
        let contents = format!(
            "last_address={}\nlast_name={}\nmaster_volume={}\nmusic_volume={}\nsfx_volume={}\n",
            self.last_address,
            self.last_name,
            self.master_volume,
            self.music_volume,
            self.sfx_volume
        );
        if let Ok(mut file) = std::fs::File::create(Self::path()) {
            let _ = file.write_all(contents.as_bytes());
        }
    }

    pub fn sfx_gain(&self) -> f32 {
        self.master_volume * self.sfx_volume
    }

    pub fn music_gain(&self) -> f32 {
        self.master_volume * self.music_volume
    }
}
