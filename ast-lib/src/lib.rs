use macroquad::prelude::{Vec2, Texture2D, FilterMode, load_texture};
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::BTreeMap;
use std::path::PathBuf;
use walkdir::WalkDir;
use futures::stream::{self, StreamExt};
use ::rand::{distributions::{Distribution, WeightedIndex}, thread_rng};

// ==== CONSTANTS ====
pub static NEXT_UID: AtomicU64 = AtomicU64::new(1);

// ==== STRUCTURES ====

#[derive(Clone, PartialEq)]
pub struct NamedTexture {
    pub texture: Texture2D,
    pub name: String,
}

// ==== TEXTURES ====

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

/// Random texture selector with strict weights
/// `custom_weights` must be provided and sum to 100.0
pub fn select_weighted_texture<'a>(
    textures: &'a BTreeMap<PathBuf, Texture2D>,
    subdir: &str,
    custom_weights: Vec<f32>,
) -> Option<NamedTexture> {
    // Filter keys to only include ones in the given subdir
    let filtered_keys: Vec<&PathBuf> = textures
        .keys()
        .filter(|k| k.to_string_lossy().contains(subdir))
        .collect();

    let amount = filtered_keys.len();
    if amount == 0 {
        return None; // no textures in this subdir
    }

    if custom_weights.len() != amount {
        panic!(
            "Number of weights ({}) does not match number of textures ({})",
            custom_weights.len(),
            amount
        );
    }

    let sum: f32 = custom_weights.iter().sum();
    if (sum - 100.0).abs() > f32::EPSILON {
        panic!("Sum of weights must be exactly 100.0, got {}", sum);
    }

    let mut rng = thread_rng();
    let dist = WeightedIndex::new(&custom_weights).unwrap();
    let selected_index = dist.sample(&mut rng);

    let selected_path = filtered_keys[selected_index];
    textures.get(selected_path).map(|tex| NamedTexture {
        texture: tex.clone(),
        name: selected_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    })
}

// ==== TRAITS ====

/// Trait defining entity behavior
pub trait CosmicEntity: Clone {
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
    fn collides_with<T: CosmicEntity>(&self, other: &T) -> bool {
        let distance = (self.get_position() - other.get_position()).length();
        distance < self.get_size() + other.get_size()
    }

    /// Find nearest entity in a slice
    fn find_nearest<'a, T: CosmicEntity>(&self, objects: &'a [T]) -> Option<Vec2> {
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

// ==== MISC ====

/// Free function to generate unique IDs
pub fn generate_uid() -> u64 {
    NEXT_UID.fetch_add(1, Ordering::Relaxed)
}

/// Change enum for adding/removing entities
pub enum Change<T> {
    Add(T),
    Remove(u64),
}

/// Apply changes to a vector of entities
pub fn apply_changes<T>(vec: &mut Vec<T>, changes: &mut Vec<Change<T>>)
where
    T: CosmicEntity,
{
    for change in changes.iter() {
        match change {
            Change::Add(item) => vec.push(item.clone_with_new_uid()),
            Change::Remove(uid) => vec.retain(|x| x.get_id() != *uid),
        }
    }
    changes.clear();
}