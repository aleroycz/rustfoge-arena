mod emitter;
mod particle;
mod presets;
mod system;

pub use emitter::EmitterConfig;
pub use particle::Particle;
pub use presets::{dust_burst, rain, snow, sparks};
pub use system::ParticleSystem;
