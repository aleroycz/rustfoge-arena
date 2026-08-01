use rodio::Source;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Sound {
    samples: Arc<[f32]>,
    channels: u16,
    sample_rate: u32,
}

impl Sound {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, rodio::decoder::DecoderError> {
        let decoder = rodio::Decoder::new(Cursor::new(bytes))?;
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();
        let samples: Vec<f32> = decoder.convert_samples().collect();
        Ok(Self {
            samples: samples.into(),
            channels,
            sample_rate,
        })
    }

    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|err| format!("reading {path:?}: {err}"))?;
        Self::from_bytes(bytes).map_err(|err| format!("decoding {path:?}: {err}"))
    }

    pub fn sine_wave(frequency_hz: f32, duration: Duration, sample_rate: u32) -> Self {
        let sample_count = (duration.as_secs_f32() * sample_rate as f32) as usize;
        let samples: Vec<f32> = (0..sample_count)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;

                let fade = fade_envelope(i, sample_count, sample_rate);
                (t * frequency_hz * std::f32::consts::TAU).sin() * fade * 0.4
            })
            .collect();
        Self {
            samples: samples.into(),
            channels: 1,
            sample_rate,
        }
    }

    pub fn chiptune_loop(sample_rate: u32) -> Self {
        const NOTES: [f32; 16] = [
            220.00, 261.63, 329.63, 440.00, 392.00, 329.63, 261.63, 220.00, 220.00, 261.63, 329.63,
            440.00, 293.66, 261.63, 220.00, 392.00,
        ];
        const NOTE_SECONDS: f32 = 0.18;

        let samples_per_note = (NOTE_SECONDS * sample_rate as f32) as usize;
        let mut samples = Vec::with_capacity(samples_per_note * NOTES.len());
        for &freq in &NOTES {
            for i in 0..samples_per_note {
                let t = i as f32 / sample_rate as f32;
                let envelope = note_envelope(i, samples_per_note);
                let wave = (t * freq * std::f32::consts::TAU).sin()
                    + 0.5 * (t * freq * 2.0 * std::f32::consts::TAU).sin()
                    + 0.25 * (t * freq * 3.0 * std::f32::consts::TAU).sin();
                samples.push(wave * envelope * 0.16);
            }
        }
        Self {
            samples: samples.into(),
            channels: 1,
            sample_rate,
        }
    }

    pub fn source(&self) -> SoundSource {
        SoundSource {
            samples: self.samples.clone(),
            position: 0,
            channels: self.channels,
            sample_rate: self.sample_rate,
        }
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f32(
            self.samples.len() as f32 / (self.channels as f32 * self.sample_rate as f32),
        )
    }
}

fn note_envelope(i: usize, len: usize) -> f32 {
    let attack = (len / 20).max(1);
    let release = (len / 6).max(1);
    if i < attack {
        i as f32 / attack as f32
    } else if i >= len - release {
        (len - i) as f32 / release as f32
    } else {
        1.0
    }
}

fn fade_envelope(i: usize, len: usize, sample_rate: u32) -> f32 {
    let fade_samples = (sample_rate / 100).max(1) as usize;
    let fade_samples = fade_samples.min(len / 2).max(1);
    if i < fade_samples {
        i as f32 / fade_samples as f32
    } else if i >= len - fade_samples {
        (len - i) as f32 / fade_samples as f32
    } else {
        1.0
    }
}

pub struct SoundSource {
    samples: Arc<[f32]>,
    position: usize,
    channels: u16,
    sample_rate: u32,
}

impl Iterator for SoundSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.samples.get(self.position).copied();
        self.position += 1;
        sample
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.samples.len().saturating_sub(self.position);
        (remaining, Some(remaining))
    }
}

impl Source for SoundSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
