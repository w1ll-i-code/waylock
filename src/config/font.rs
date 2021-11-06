use font_loader::system_fonts::{FontPropertyBuilder, get};
use std::fmt::Display;
use std::ops::Deref;
use fontdue::Font;
use log::error;

pub fn load_font<T: Deref<Target=str> + Display >(font: T) -> Option<Font> {
    let properties = FontPropertyBuilder::new()
        .family(&font)
        .build();

    match get(&properties) {
        None => {
            eprintln!("Chosen font {} not found.", font);
            error!("Font \"{}\" not available.", font);
            None
        },
        Some((data, _c_int)) => match Font::from_bytes(data, Default::default()) {
            Ok(font) => Some(font),
            Err(err) => {
                error!("Could not load Font {}. ERROR: {}", font, err);
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use font_loader::system_fonts::{FontPropertyBuilder, get};
    use fontdue::Font;

    fn default_font_should_exist() {
        let properties = FontPropertyBuilder::new()
            .family("monospace")
            .build();

        let (data, _c_int) = get(&properties)
            .expect("Font should exist");

        let _font = Font::from_bytes(data, Default::default())
            .expect("Font should not be corrupted");
    }
}