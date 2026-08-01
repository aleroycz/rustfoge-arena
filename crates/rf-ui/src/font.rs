use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct GlyphUv {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

pub struct FontAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub glyph_aspect: f32,
    glyphs: HashMap<char, GlyphUv>,
}

impl FontAtlas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        const COLS: u32 = 16;
        const CELL: u32 = 8;
        let entries = glyph_table();
        let rows = (entries.len() as u32).div_ceil(COLS);
        let width = COLS * CELL;
        let height = rows * CELL;

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let mut glyphs = HashMap::with_capacity(entries.len());

        for (index, (ch, rows_bits)) in entries.iter().enumerate() {
            let col = index as u32 % COLS;
            let row = index as u32 / COLS;
            let base_x = col * CELL;
            let base_y = row * CELL;

            for (dy, bits) in rows_bits.iter().enumerate() {
                for dx in 0..5u32 {
                    let on = (bits >> (4 - dx)) & 1 == 1;
                    if !on {
                        continue;
                    }
                    let x = base_x + dx;
                    let y = base_y + dy as u32;
                    let idx = ((y * width + x) * 4) as usize;
                    pixels[idx] = 255;
                    pixels[idx + 1] = 255;
                    pixels[idx + 2] = 255;
                    pixels[idx + 3] = 255;
                }
            }

            let uv_min = [base_x as f32 / width as f32, base_y as f32 / height as f32];
            let uv_max = [
                (base_x + 5) as f32 / width as f32,
                (base_y + 7) as f32 / height as f32,
            ];
            glyphs.insert(*ch, GlyphUv { uv_min, uv_max });
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui font atlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            glyph_aspect: 5.0 / 7.0,
            glyphs,
        }
    }

    pub fn glyph(&self, ch: char) -> Option<GlyphUv> {
        self.glyphs.get(&ch.to_ascii_uppercase()).copied()
    }
}

type Rows = [u8; 7];

fn glyph_table() -> Vec<(char, Rows)> {
    vec![
        (' ', [0, 0, 0, 0, 0, 0, 0]),
        (
            '0',
            [
                0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
            ],
        ),
        (
            '1',
            [
                0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ],
        ),
        (
            '2',
            [
                0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
            ],
        ),
        (
            '3',
            [
                0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
            ],
        ),
        (
            '4',
            [
                0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
            ],
        ),
        (
            '5',
            [
                0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
            ],
        ),
        (
            '6',
            [
                0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
            ],
        ),
        (
            '7',
            [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
            ],
        ),
        (
            '8',
            [
                0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
            ],
        ),
        (
            '9',
            [
                0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
            ],
        ),
        (
            'A',
            [
                0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ],
        ),
        (
            'B',
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
            ],
        ),
        (
            'C',
            [
                0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
            ],
        ),
        (
            'D',
            [
                0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
            ],
        ),
        (
            'E',
            [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
            ],
        ),
        (
            'F',
            [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
        ),
        (
            'G',
            [
                0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
            ],
        ),
        (
            'H',
            [
                0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ],
        ),
        (
            'I',
            [
                0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ],
        ),
        (
            'J',
            [
                0b00001, 0b00001, 0b00001, 0b00001, 0b10001, 0b10001, 0b01110,
            ],
        ),
        (
            'K',
            [
                0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
            ],
        ),
        (
            'L',
            [
                0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
            ],
        ),
        (
            'M',
            [
                0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
            ],
        ),
        (
            'N',
            [
                0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
            ],
        ),
        (
            'O',
            [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
        ),
        (
            'P',
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
        ),
        (
            'Q',
            [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
            ],
        ),
        (
            'R',
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
            ],
        ),
        (
            'S',
            [
                0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
        ),
        (
            'T',
            [
                0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
            ],
        ),
        (
            'U',
            [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
        ),
        (
            'V',
            [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
            ],
        ),
        (
            'W',
            [
                0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
            ],
        ),
        (
            'X',
            [
                0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
            ],
        ),
        (
            'Y',
            [
                0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
            ],
        ),
        (
            'Z',
            [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
            ],
        ),
        ('.', [0, 0, 0, 0, 0, 0b01100, 0b01100]),
        (':', [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0]),
        ('-', [0, 0, 0, 0b11111, 0, 0, 0]),
        (
            '%',
            [
                0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011,
            ],
        ),
        (
            '/',
            [
                0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
            ],
        ),
        ('°', [0b01100, 0b10010, 0b10010, 0b01100, 0, 0, 0]),
        (
            '!',
            [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100],
        ),
    ]
}
