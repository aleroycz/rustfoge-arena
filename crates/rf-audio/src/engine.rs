use glam::Vec3;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source, SpatialSink};
use thiserror::Error;

use crate::Sound;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("failed to open audio output device: {0}")]
    Stream(#[from] rodio::StreamError),
    #[error("failed to create playback sink: {0}")]
    Play(#[from] rodio::PlayError),
}

pub struct AudioEngine {
    _stream: OutputStream,
    handle: OutputStreamHandle,

    ear_offset: Vec3,
}

impl AudioEngine {
    pub fn new() -> Result<Self, AudioError> {
        let (stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream: stream,
            handle,
            ear_offset: Vec3::X * 0.2,
        })
    }

    pub fn play(&self, sound: &Sound, volume: f32) -> Result<Sink, AudioError> {
        let sink = Sink::try_new(&self.handle)?;
        sink.set_volume(volume);
        sink.append(sound.source());
        Ok(sink)
    }

    pub fn play_looped(&self, sound: &Sound, volume: f32) -> Result<Sink, AudioError> {
        let sink = Sink::try_new(&self.handle)?;
        sink.set_volume(volume);
        sink.append(sound.source().repeat_infinite());
        Ok(sink)
    }

    pub fn play_spatial(
        &self,
        sound: &Sound,
        emitter_pos: Vec3,
        listener_pos: Vec3,
    ) -> Result<SpatialSink, AudioError> {
        let sink = SpatialSink::try_new(
            &self.handle,
            emitter_pos.into(),
            (listener_pos - self.ear_offset).into(),
            (listener_pos + self.ear_offset).into(),
        )?;
        sink.append(sound.source());
        Ok(sink)
    }

    pub fn update_listener(&self, sink: &SpatialSink, listener_pos: Vec3) {
        sink.set_left_ear_position((listener_pos - self.ear_offset).into());
        sink.set_right_ear_position((listener_pos + self.ear_offset).into());
    }
}
