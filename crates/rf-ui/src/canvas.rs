#[derive(Debug, Clone, Copy)]
pub struct RectCommand {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub radius: f32,
}

#[derive(Debug, Clone)]
pub struct TextCommand {
    pub pos: [f32; 2],
    pub text: String,

    pub scale: f32,
    pub color: [f32; 4],
}

#[derive(Default)]
pub struct Canvas {
    pub width: f32,
    pub height: f32,
    pub(crate) rects: Vec<RectCommand>,
    pub(crate) texts: Vec<TextCommand>,
}

impl Canvas {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            rects: Vec::new(),
            texts: Vec::new(),
        }
    }

    pub fn clear(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.rects.clear();
        self.texts.clear();
    }

    pub fn rect(&mut self, pos: [f32; 2], size: [f32; 2], color: [f32; 4], radius: f32) {
        self.rects.push(RectCommand {
            pos,
            size,
            color,
            radius,
        });
    }

    pub fn text(&mut self, pos: [f32; 2], text: impl Into<String>, scale: f32, color: [f32; 4]) {
        self.texts.push(TextCommand {
            pos,
            text: text.into(),
            scale,
            color,
        });
    }

    pub fn bar(
        &mut self,
        pos: [f32; 2],
        size: [f32; 2],
        fraction: f32,
        track_color: [f32; 4],
        fill_color: [f32; 4],
        cut: f32,
    ) {
        self.rect(pos, size, track_color, cut);
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction > 0.0 {
            self.rect(pos, [size[0] * fraction, size[1]], fill_color, cut);
        }
    }

    pub fn frame(
        &mut self,
        pos: [f32; 2],
        size: [f32; 2],
        border_color: [f32; 4],
        fill_color: [f32; 4],
        border_width: f32,
        cut: f32,
    ) {
        self.rect(pos, size, border_color, cut);
        let inner_cut = (cut - border_width).max(0.0);
        self.rect(
            [pos[0] + border_width, pos[1] + border_width],
            [size[0] - border_width * 2.0, size[1] - border_width * 2.0],
            fill_color,
            inner_cut,
        );
    }

    pub fn text_advance(scale: f32) -> f32 {
        let glyph_width = scale * 5.0 / 7.0;
        glyph_width + scale * (1.0 / 7.0)
    }

    pub fn text_width(text: &str, scale: f32) -> f32 {
        let advance = Self::text_advance(scale);
        (text.chars().count() as f32 * advance - scale * (1.0 / 7.0)).max(0.0)
    }
}
