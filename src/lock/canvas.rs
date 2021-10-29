use fontdue::layout::Layout;
use fontdue::Font;

pub struct Canvas<'a> {
    pub mem: *mut u8,
    pub dimensions: (usize, usize),
    pub color: u32,
    pub fonts: &'a [Font],
}

impl Canvas<'_> {
    pub fn fill(&self) {
        let size = self.dimensions.0 * self.dimensions.1;
        let buf = unsafe { std::slice::from_raw_parts_mut(self.mem as *mut u32, size as usize) };
        buf.fill(self.color);
    }

    pub fn draw_square(&self, from: (usize, usize), to: (usize, usize)) {
        let (from_x, from_y) = from;
        let (to_x, to_y) = to;

        let size = self.dimensions.0 * self.dimensions.1;
        let buf = unsafe { std::slice::from_raw_parts_mut(self.mem as *mut u32, size as usize) };

        for y in from_y..to_y {
            let y_off = y * self.dimensions.0;
            for x in (y_off + from_x)..(y_off + to_x) {
                buf[x as usize] = self.color
            }
        }
    }

    fn draw_bitmap(&self, bitmap: &[u8], dimensions: (usize, usize), position: (usize, usize)) {
        let (x_pos, y_pos) = position;
        let (x_dim, y_dim) = dimensions;

        let size = self.dimensions.0 * self.dimensions.1;
        let buf = unsafe { std::slice::from_raw_parts_mut(self.mem as *mut u32, size as usize) };
        let mut src = bitmap.iter();

        for i in 0..y_dim {
            let mut buf_offset = ((y_pos + i) * self.dimensions.0 + x_pos) as usize;
            for _ in 0..x_dim {
                let alpha = *src.next().unwrap() as u32;
                let re_alpha = 255 - alpha;
                let mut new = self.color.to_ne_bytes();
                let current = buf[buf_offset].to_ne_bytes();

                for i in 0..4 {
                    new[i] = ((new[i] as u32 * alpha + current[i] as u32 * re_alpha) >> 8) as u8;
                }

                buf[buf_offset] = u32::from_ne_bytes(new);
                buf_offset += 1;
            }
        }
    }

    pub fn draw_layout(&self, layout: &mut Layout) {
        let glyphs = layout.glyphs();

        for glyph in glyphs {
            if glyph.char_data.is_control() || glyph.char_data.is_whitespace() {
                continue;
            }

            let (metrics, buf) = self.fonts[0].rasterize_config(glyph.key);
            self.draw_bitmap(
                &buf,
                (metrics.width, metrics.height),
                (glyph.x as usize, glyph.y as usize),
            )
        }
    }
}
