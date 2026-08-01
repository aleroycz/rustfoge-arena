use anyhow::{Context, Result};
use rf_renderer::GpuTexture;

pub fn load_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: impl AsRef<std::path::Path>,
) -> Result<GpuTexture> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).with_context(|| format!("reading texture {path:?}"))?;
    load_texture_from_bytes(device, queue, &bytes, &path.to_string_lossy())
}

pub fn load_texture_from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bytes: &[u8],
    label: &str,
) -> Result<GpuTexture> {
    let image = image::load_from_memory(bytes)
        .with_context(|| format!("decoding image {label}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok(GpuTexture::from_rgba8(
        device, queue, &image, width, height, label,
    ))
}
