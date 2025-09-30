use macroquad::prelude::{FilterMode, Image, Texture2D};
use std::{collections::BTreeMap, path::PathBuf};
use ast_lib::{NamedTexture, load_textures_recursive_parallel};

use once_cell::sync::Lazy;

// Make modules public
pub mod asteroid;
pub mod spaceship;
pub mod missile;

pub mod floating_text;
pub mod menus;
pub mod gamestate;
pub mod key_bindings;

/// Textures

pub static MISSING_TEXTURE: Lazy<NamedTexture> = Lazy::new(|| {
    let pixels: Vec<u8> = vec![
        255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
    ];
    let image = Image {
        bytes: pixels,
        width: 2,
        height: 2,
    };
    let tex = Texture2D::from_image(&image);
    tex.set_filter(FilterMode::Nearest);

    NamedTexture {
        texture: tex,
        name: "MISSING_TEXTURE".to_string(),
    }
});

pub static TEXTURE_SET: Lazy<BTreeMap<PathBuf, Texture2D>> = Lazy::new(|| {
    pollster::block_on(async {
        load_textures_recursive_parallel(PathBuf::from("./assets/textures")).await
    })
});
