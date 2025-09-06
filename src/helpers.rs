use macroquad::prelude::{load_texture, FilterMode, Image, Texture2D, Vec2};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{collections::BTreeMap, path::PathBuf};

use futures::stream::{self, StreamExt};
use once_cell::sync::Lazy;
use walkdir::WalkDir;

pub static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// Free function to generate unique IDs
pub fn generate_uid() -> u64 {
    NEXT_UID.fetch_add(1, Ordering::Relaxed)
}

/// Trait defining entity behavior
pub trait Entity: Clone {
    fn get_id(&self) -> u64;
    fn clone_with_new_uid(&self) -> Self;
    fn get_position(&self) -> Vec2;
    fn get_speed(&self) -> f32;
    fn get_size(&self) -> f32;
    fn get_rotation(&self) -> f32;
    fn add_rotation(&mut self, amount: f32);

    /// Is the entity out of bounds
    fn is_out_of_bounds(&self, bounds: &Vec2) -> bool {
        let pos = self.get_position();
        pos.x < 0.0 || pos.x > bounds.x || pos.y < 0.0 || pos.y > bounds.y
    }

    /// Check collision with another entity
    fn collides_with<T: Entity>(&self, other: &T) -> bool {
        let distance = (self.get_position() - other.get_position()).length();
        distance < self.get_size() + other.get_size()
    }

    /// Find nearest entity in a slice
    fn find_nearest<'a, T: Entity>(&self, objects: &'a [T]) -> Option<Vec2> {
        let mut nearest = None;
        let mut min_distance = std::f32::INFINITY;
        let pos = self.get_position();

        for obj in objects {
            let distance = obj.get_position().distance(pos);
            if distance < min_distance {
                min_distance = distance;
                nearest = Some(obj.get_position());
            }
        }

        nearest
    }
}

/// Change enum for adding/removing entities
pub enum Change<T> {
    Add(T),
    Remove(u64),
}

/// Apply changes to a vector of entities
pub fn apply_changes<T>(vec: &mut Vec<T>, changes: &mut Vec<Change<T>>)
where
    T: Entity,
{
    for change in changes.iter() {
        match change {
            Change::Add(item) => vec.push(item.clone_with_new_uid()),
            Change::Remove(uid) => vec.retain(|x| x.get_id() != *uid),
        }
    }
    changes.clear();
}

/// Macro to implement Entity for a type with standard fields
#[macro_export]
macro_rules! import_entity {
    ($t:ty) => {
        use macroquad::prelude::{draw_text, vec2, Vec2};
        use std::f32::consts::PI;

        impl $crate::helpers::Entity for $t {
            fn get_id(&self) -> u64 {
                self.id
            }

            fn clone_with_new_uid(&self) -> Self {
                let mut cloned = self.clone();
                cloned.id = $crate::helpers::generate_uid();
                cloned
            }

            fn get_position(&self) -> ::macroquad::prelude::Vec2 {
                self.position
            }

            fn get_speed(&self) -> f32 {
                self.speed
            }

            fn get_size(&self) -> f32 {
                self.size
            }

            fn get_rotation(&self) -> f32 {
                self.rotation
            }

            fn add_rotation(&mut self, amount: f32) {
                self.rotation = (self.rotation - amount) % (std::f32::consts::PI * 2.0);
            }
        }
    };
}

/// Textures
pub static MISSING_TEXTURE: Lazy<Texture2D> = Lazy::new(|| {
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
    tex
});

pub static TEXTURE_SET: Lazy<BTreeMap<PathBuf, Texture2D>> = Lazy::new(|| {
    pollster::block_on(async {
        load_textures_recursive_parallel(PathBuf::from("./assets/textures")).await
    })
});

/// Load textures recursively
pub async fn load_textures_recursive_parallel(root: PathBuf) -> BTreeMap<PathBuf, Texture2D> {
    let paths: Vec<PathBuf> = WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let p = e.into_path();
            match p
                .extension()
                .and_then(|x| x.to_str())
                .map(|s| s.to_lowercase())
            {
                Some(ext) if ["png", "jpg", "jpeg"].contains(&ext.as_str()) => Some(p),
                _ => None,
            }
        })
        .collect();

    let concurrency = 8usize;
    let loaded_vec = stream::iter(paths.into_iter().map(|path| {
        let root_clone = root.clone();
        async move {
            let rel_path = path.strip_prefix(&root_clone).unwrap().to_path_buf();
            let tex = load_texture(path.to_string_lossy().as_ref())
                .await
                .expect(&format!("Failed to load: {:?}", rel_path));

            if rel_path.ends_with("background2.png") {
                tex.set_filter(FilterMode::Nearest);
            }

            println!("[INFO] Loaded texture: {:?}", rel_path);
            (rel_path, tex)
        }
    }))
    .buffer_unordered(concurrency)
    .collect::<Vec<_>>()
    .await;

    let mut textures = BTreeMap::new();
    for (k, v) in loaded_vec {
        textures.insert(k, v);
    }
    textures
}
