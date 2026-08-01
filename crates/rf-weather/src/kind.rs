#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherKind {
    Clear,
    Cloudy,
    Rain,
    Snow,
    Storm,
}

impl WeatherKind {
    pub(crate) fn base_params(self) -> (f32, f32, f32, f32, f32, f32) {
        match self {
            WeatherKind::Clear => (0.1, 0.004, 0.0, 0.0, 0.0, 1.0),
            WeatherKind::Cloudy => (0.6, 0.008, 0.05, 0.0, 0.0, 0.85),
            WeatherKind::Rain => (0.8, 0.02, 0.2, 1.0, 0.0, 0.6),
            WeatherKind::Snow => (0.7, 0.015, 0.1, 0.0, 1.0, 0.75),
            WeatherKind::Storm => (0.95, 0.035, 0.85, 1.0, 0.0, 0.35),
        }
    }

    pub(crate) fn candidates(self) -> &'static [WeatherKind] {
        match self {
            WeatherKind::Clear => &[WeatherKind::Clear, WeatherKind::Cloudy],
            WeatherKind::Cloudy => &[
                WeatherKind::Clear,
                WeatherKind::Cloudy,
                WeatherKind::Rain,
                WeatherKind::Snow,
            ],
            WeatherKind::Rain => &[WeatherKind::Cloudy, WeatherKind::Rain, WeatherKind::Storm],
            WeatherKind::Snow => &[WeatherKind::Cloudy, WeatherKind::Snow],
            WeatherKind::Storm => &[WeatherKind::Rain, WeatherKind::Storm, WeatherKind::Cloudy],
        }
    }
}
