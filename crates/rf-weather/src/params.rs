use rf_math::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct WeatherParams {
    pub cloud_coverage: f32,
    pub fog_density: f32,
    pub storm_darkness: f32,

    pub rain_intensity: f32,

    pub snow_intensity: f32,

    pub ambient_dimming: f32,
    pub wind: Vec3,
}
