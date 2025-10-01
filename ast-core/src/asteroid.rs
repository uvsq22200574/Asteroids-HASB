use ast_lib::{CosmicEntity, NamedTexture, Change, generate_uid, select_weighted_texture, MISSING_TEXTURE, TEXTURE_SET};
use mac_der::Entity;
use std::f32::consts::PI;
use macroquad::prelude::{
    draw_circle_lines, draw_line, draw_texture_ex, draw_text, measure_text, screen_dpi_scale, screen_height,
    screen_width, vec2, DrawTextureParams, Vec2, BLUE, GREEN, RED, WHITE, YELLOW,
};
use ::rand::{thread_rng, Rng};


#[derive(PartialEq, Clone, Entity)]
pub struct Asteroid {
    id: u64,
    position: Vec2,
    speed: f32,
    size: f32,
    rotation: f32,
    direction: f32,
    speed_multiplier: f32,
    turn_rate: f32,
    texture: NamedTexture,
}

impl Asteroid {
    /// The scale of an asteroid so that it's not too small
    /// Affects both the visual and the physics
    pub const SCALE: f32 = 30.0;

    /// Default constructor using static TEXTURE_SET
    pub fn new_default() -> Self {
        Self::new(None, None, None, None, None, None, None, None)
    }

    /// Main constructor
    pub fn new(
        position: Option<Vec2>,
        speed: Option<f32>,
        size: Option<f32>,
        rotation: Option<f32>,
        direction: Option<f32>,
        speed_multiplier: Option<f32>,
        turn_rate: Option<f32>,
        texture: Option<NamedTexture>,
    ) -> Self {
        let mut rng = thread_rng();
        let new_properties = Self::new_properties();

        // Default values
        let default_position = position.unwrap_or_else(|| Self::new_alea_pos(30.0));
        let default_speed = speed.unwrap_or(new_properties.2);
        let default_size = size.unwrap_or(rng.gen_range(2..=3) as f32 * Self::SCALE);
        let default_rotation = rotation.unwrap_or(Self::new_rotation());
        let default_direction =
            direction.unwrap_or(rng.gen_range(0.0..=2.0 * PI));
        let default_speed_multiplier = speed_multiplier.unwrap_or(new_properties.1);
        let default_turn_rate = turn_rate.unwrap_or(rng.gen_range(0.5..1.5) * if rng.gen_bool(0.5) { 1.0 } else { -1.0 });

        // Texture selection:
        let default_texture = texture
            .or_else(|| select_weighted_texture(&TEXTURE_SET, "asteroid/", vec![85.0, 10.0, 5.0]))
            .unwrap_or_else(|| MISSING_TEXTURE.clone());

        Self {
            id: generate_uid(),
            position: default_position,
            speed: default_speed,
            size: default_size,
            rotation: default_rotation,
            direction: default_direction,
            speed_multiplier: default_speed_multiplier,
            turn_rate: default_turn_rate,
            texture: default_texture,
        }
    }

    pub fn get_direction(&self) -> f32 {
        self.direction
    }

    pub fn get_texture(&self) -> &NamedTexture {
        &self.texture
    }

    pub fn get_speed_multiplier(&self) -> f32 {
        self.speed_multiplier
    }

    pub fn get_turn_rate(&self) -> f32 {
        self.turn_rate
    }

    pub fn compute_score(&self, base: u128, multipliers: &Vec<u8>, size: Option<f32>) -> u128 {
        let index = ((size.unwrap_or(self.get_size()) / Self::SCALE) - 1.0) as usize;
        base * multipliers[index] as u128
    }

    // Moves the object based on its speed, applying inertia.
    pub fn update(&mut self, delta_time: f64) {
        let direction = vec2(self.direction.cos(), self.direction.sin());
        self.rotation += self.turn_rate * delta_time as f32;
        self.position += direction * self.speed * self.get_speed_multiplier() * delta_time as f32;
        // Move at the opposite edge
        self.position = Self::bound_pos(self.position);
    }

    /// Generates a random position near one of the screen edges.
    fn new_alea_pos(offset: f32) -> Vec2 {
        let mut rng = thread_rng();
        let nearpos: f32 = rng.gen_range(offset * 0.5..=offset);
        // 1 = top, 2 = right, 3 = bottom, 4 = left
        let nearside = rng.gen_range(1..=4);
        let xpos: f32 = match nearside {
            2 => screen_width() - nearpos,
            4 => nearpos,
            _ => rng.gen_range(0.0..=screen_width()),
        };
        let ypos: f32 = match nearside {
            1 => nearpos,
            3 => screen_height() - nearpos,
            _ => rng.gen_range(0.0..=screen_height()),
        };
        vec2(xpos, ypos)
    }

    /// Create properties based on each other and assign them to a tuple for the constructor
    fn new_properties() -> (f32, f32, f32) {
        let mut rng = thread_rng();
        let size = rng.gen_range(1..=3) as f32 * Self::SCALE;
        let speed_multiplier = rng.gen_range(0.4..=1.5);
        let size_to_speed = match size {
            10.0 => 3.5,
            20.0 => 2.0,
            _ => 1.0,
        };

        (
            size,
            speed_multiplier,
            size_to_speed * speed_multiplier * 120.0,
        )
    }

    fn new_rotation() -> f32 {
        let mut rng = thread_rng();
        rng.gen_range(1.0..=2.0 * PI)
    }

    fn bound_pos(mut pos: Vec2) -> Vec2 {
        pos.x = Self::bound_to(pos.x, screen_width());
        pos.y = Self::bound_to(pos.y, screen_height());
        pos
    }

    fn bound_to(coord: f32, max: f32) -> f32 {
        if coord < 0.0 {
            max
        } else if coord > max {
            0.0
        } else {
            coord
        }
    }

    // Create two smaller asteroids moving forward based on rotation
    pub fn split(&self, can_add: bool, to_add: u8, change_list: &mut Vec<Change<Asteroid>>) {
        let mut rng = thread_rng();
        let new_size = self.get_size() - Self::SCALE;

        if new_size <= 0.0 {
            change_list.push(Change::Remove(self.id));
            return;
        }

        // Determine how many asteroids to create
        let num_to_create = if can_add { to_add } else { 1 };
        let speed_factor = rng.gen_range(1.0..=1.5);

        // Compute evenly spread angles around the opposite of current rotation
        let base_rotation = -self.get_rotation();
        let angle_step = PI / (num_to_create as f32); // spread children over ~180 degrees
        let start_angle = base_rotation - PI / 2.0; // center the spread

        for i in 0..num_to_create {
            let speed = self.get_speed() * speed_factor;

            // Rotation for this child
            let rotation = start_angle + angle_step * (i as f32 + 0.5);

            // Turn rate (randomized)
            // Base magnitude of parent's turn rate
            let base_magnitude = self.get_turn_rate().abs();

            // Random factor between 0.5 and 1.5 for "speed up or slow down"
            let factor = rng.gen_range(0.5..=1.5);

            // Randomly flip direction
            let sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };

            let turn_rate = base_magnitude * factor * sign;

            // Compute direction vector based on rotation
            let direction_vec = Vec2::new(rotation.cos(), rotation.sin()) * 0.33 * self.size;

            // Create the new asteroid
            let new_asteroid = Asteroid::new(
                Some(self.get_position() + direction_vec),
                Some(speed),
                Some(new_size),
                Some(rotation),
                Some(if i % 2 == 0 { -1.0 } else { 1. } * self.direction),
                Some(self.speed_multiplier),
                Some(turn_rate),
                Some(self.texture.clone()),
            );

            change_list.push(Change::Add(new_asteroid));
        }

        // Always remove the original asteroid
        change_list.push(Change::Remove(self.id));
    }

    pub fn grant_score(&self, score: &mut u128, multipliers: &Vec<u8>) -> u128 {
        let award = self.compute_score(100, multipliers, Some(self.size));
        *score += award;
        award
    }

    pub fn draw_trajectory(&self) {
        // Define the arrow length and compute the direction where the asteroid is moving
        let arrow_length = 40.0;
        // Normalize to get direction

        // Get the asteroid's current position and rotation
        let start = self.get_position();

        // Calculate the direction of the arrow based on the asteroid's rotation
        let direction = vec2(self.get_direction().cos(), self.get_direction().sin())
            * self.get_direction().signum();
        let rotation = vec2(self.get_rotation().cos(), -self.get_rotation().sin());

        // Calculate the end point of the arrows
        let end_rotation = start + rotation * arrow_length;
        let end_direction = start + direction * arrow_length;

        // Draw the trajectory arrow
        draw_line(start.x, start.y, end_direction.x, end_direction.y, 2.0, RED);
        // Draw the rotation direction of the texture arrow
        draw_line(
            start.x,
            start.y,
            end_rotation.x,
            end_rotation.y,
            2.0,
            YELLOW,
        );
    }

    pub fn draw_self(&self, debug: bool) {
        let font_size = 20.0;
        let position = self.get_position();
        let draw_pos = position - self.size; // correct centering

        draw_texture_ex(
            &self.texture.texture,
            // Center the texture to the asteroid's center
            draw_pos.x,
            draw_pos.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(self.size, self.size) * 2.0),
                rotation: -self.get_rotation(),
                ..Default::default()
            },
        );

        if debug {
            // Attributes
            let size_to_speed = match self.get_size() {
                30.0 => 3.5,
                60.0 => 2.0,
                _ => 1.0,
            };
            let mut texts = Vec::from([
                format!("x:{:.2} y:{:.2}", position.x, position.y),
                format!("Size:{}", self.get_size()),
                format!(
                    "Speed:{:.2}px/s",
                    self.get_speed() * self.get_speed_multiplier() * size_to_speed
                ),
                format!(
                    "Rotation:{}|{:.3}rad",
                    if self.get_turn_rate().signum() == 1.0 {
                        "L"
                    } else {
                        "R"
                    },
                    self.get_rotation()
                ),
                format!("Turn Rate:{:.3}rad/s", self.get_turn_rate()),
                format!("Direction: {:.3}rad", self.get_direction()),
                format!(
                    "Speed modifier:{:.2}%",
                    (self.get_speed_multiplier() * 100.0)
                ),
                format!("Variant:{}", self.get_texture().name),
                format!("UID: {}", self.id),
            ]);

            let mut debug_text_sizes: Vec<u16> = Vec::new();

            for field in &texts {
                debug_text_sizes.push(
                    measure_text(
                        &field.to_string(),
                        None,
                        font_size as u16,
                        screen_dpi_scale(),
                    )
                    .width as u16,
                );
            }

            let text_size = *debug_text_sizes.iter().max().unwrap() as f32;

            // Draw besides the asteroid
            let x_offset = if screen_width() - position.x >= text_size + Self::SCALE {
                self.get_size() + 5.0
            } else {
                -text_size - self.get_size()
            };
            let y_offset = if screen_height() - position.y >= font_size * texts.len() as f32 {
                20.0
            } else {
                -font_size * texts.len() as f32
            };

            // Hitbox
            draw_circle_lines(position.x, position.y, self.get_size(), 1.0, BLUE);
            // Center
            draw_circle_lines(position.x, position.y, 3.0, 1.5, BLUE);

            for (index, field) in &mut texts.iter_mut().enumerate() {
                draw_text(
                    field,
                    position.x + x_offset,
                    (position.y + y_offset) + index as f32 * 20.0,
                    font_size,
                    GREEN,
                );
            }

            // Trajectory + Rotation
            self.draw_trajectory();
            // Comparison line
            draw_line(
                self.position.x,
                self.position.y,
                self.position.x,
                self.position.y - 75.0,
                1.0,
                WHITE,
            );
        }
    }
}
