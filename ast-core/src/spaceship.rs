use ast_lib::generate_uid;
use mac_der::Entity;
use ast_lib::CosmicEntity;

use macroquad::prelude::{
    draw_circle, draw_circle_lines, draw_line, draw_triangle, draw_text, measure_text, screen_dpi_scale,
    screen_height, screen_width, vec2, Color, Vec2, BLUE, LIME, PINK, RED, YELLOW,
};
use std::f32::consts::PI;

#[derive(Clone, Copy, Entity)]
pub struct Spaceship {
    id: u64,
    position: Vec2,
    speed: f32,
    max_speed: f32,
    rotation: f32,
    turn_rate: f32,
    missile_capacity: u8,
    special_radius: f32,
    size: f32,
    shield: f32,
    shield_timer: f64,
    invulnerability: f64,
    alive: bool,
    hom_cooldown: f64,
    fire_cooldown: f64,
}

#[allow(unused)]
impl Spaceship {
    pub fn new() -> Self {
        Spaceship {
            id: generate_uid(),
            position: vec2(screen_width() / 2.0, screen_height() / 2.0),
            speed: 0.0,
            max_speed: 500.0,
            rotation: 0.0,
            turn_rate: 4.0,
            missile_capacity: 2,
            special_radius: 55.0,
            size: 25.0,
            shield: 100.0,
            shield_timer: 0.0,
            invulnerability: 3.0,
            alive: true,
            hom_cooldown: 0.0,
            fire_cooldown: 0.0,
        }
    }

    /// Returns a position `distance` units in front of the spaceship.
    pub fn position_in_front_with_rotation(&self, distance: f32, rotation_offset: f32) -> Vec2 {
        Vec2::new(
            (self.rotation + rotation_offset).cos() * distance,
            -(self.rotation + rotation_offset).sin() * distance,
        ) + self.position
    }

    // Rotate a point based on the rotation
    fn rotate_point(&self, point: Vec2, rotation_angle: f32) -> Vec2 {
        let cos_angle = rotation_angle.cos();
        let sin_angle = rotation_angle.sin();

        Vec2::new(
            point.x * cos_angle - point.y * sin_angle,
            point.x * sin_angle + point.y * cos_angle,
        )
    }

    pub fn draw_trajectory(&self, length: Option<f32>, rotation_angle: Option<f32>) {
        let length = length.unwrap_or(8000.0);
        let rotation_angle = rotation_angle.unwrap_or(0.0);

        // Compute end point using the spaceship helper function
        let end_point = self.position_in_front_with_rotation(length, rotation_angle);

        // Draw the trajectory arrow
        draw_line(
            self.position.x,
            self.position.y,
            end_point.x,
            end_point.y,
            2.0,
            Color::from_rgba(255, 255, 255, 64),
        );
    }

    // Draw the spaceship and its shield
    pub fn draw(&mut self, size: f32, delta_time: f64, debug: bool) {
        let position = self.get_position();

        // === Spaceship triangle ===
        let height = size * (PI / 3.0).cos();

        let front = Vec2::new(size, 0.0);
        let left = Vec2::new(-size / 2.0, height);
        let right = Vec2::new(-size / 2.0, -height);

        let rotated_front = self.rotate_point(front, -self.rotation);
        let rotated_left = self.rotate_point(left, -self.rotation);
        let rotated_right = self.rotate_point(right, -self.rotation);

        if !debug {
            draw_triangle(
                self.position + rotated_front,
                self.position + rotated_left,
                self.position + rotated_right,
                YELLOW,
            );
        }

        // === Shield rings based on strength ===
        let shield_strength = self.shield;
        if shield_strength > 0.0 {
            draw_circle_lines(
                position.x,
                position.y,
                self.size + 2.0,
                5.0,
                Color::from_rgba(255, 0, 0, ((shield_strength) / 33.3 * 255.0) as u8),
            );
        }
        if shield_strength > 33.0 {
            draw_circle_lines(
                position.x,
                position.y,
                self.size + 7.0,
                5.0,
                Color::from_rgba(255, 255, 0, ((shield_strength - 33.3) / 33.3 * 255.0) as u8),
            );
        }
        if shield_strength > 66.0 {
            draw_circle_lines(
                position.x,
                position.y,
                self.size + 13.0,
                5.0,
                Color::from_rgba(0, 255, 0, ((shield_strength - 66.6) / 33.3 * 255.0) as u8),
            );
        }

        // === Blinking white shield (on top of the rings) ===
        if self.get_invulnerability() > 0.0 {
            // Update shield timer
            self.shield_timer += delta_time;

            // Optional: wrap to prevent it from growing too large
            let blink_period = 0.75; // seconds per cycle
            self.shield_timer %= blink_period;

            // Compute sine for alpha
            let sine = (self.shield_timer / blink_period * 2.0 * std::f64::consts::PI).sin();
            let alpha = ((sine * 0.5 + 0.5) * (255.0 - 64.0) + 64.0) as u8;

            draw_circle_lines(
                position.x,
                position.y,
                self.size,
                20.0,
                Color::from_rgba(128, 255, 255, alpha),
            );
        }

        // === Debug rendering ===
        if debug {
            // Hitbox
            draw_circle_lines(position.x, position.y, self.size, 3.0, BLUE);

            // Direction line
            self.draw_trajectory(Some(4000.0), Some(0.0));

            // use full real ranges, same as you normally do
            let positions = self.generate_positions_angles(
                std::f32::consts::PI / 2.0 + 0.2,
                3.0 * std::f32::consts::PI / 2.0,
                std::f32::consts::PI / 2.0,
                3.0 * std::f32::consts::PI / 2.0 - 0.2,
            );

            let total = self.missile_capacity;
            let half = total / 2; // integer division

            for i in 0..total {
                let (pos, _) = positions[i as usize]; // get the position corresponding to this index
                let color = if i < half { LIME } else { RED };
                draw_circle(pos.x, pos.y, 5.0, color);
            }

            draw_circle(self.get_position().x, self.get_position().y, 7.5, YELLOW);

            let font_size = 20.0;
            let mut texts = Vec::from([
                format!("x: {:.2} y: {:.2}", position.x, position.y),
                format!("Velocity:{:.2}px/s", self.get_speed()),
                format!("Rotation:{:.2}rad", self.get_rotation()),
                format!("Capacity:{}", self.get_missile_capacity()),
                format!("Shield:{}", self.get_shield()),
                format!("I-frames: {}", self.get_invulnerability()),
                format!("F-cool: {}", self.get_firing_cooldown()),
                format!("H-cool: {}", self.get_homming_cooldown()),
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

            let x_offset = if screen_width() - position.x >= text_size + 25.0 {
                25.0
            } else {
                -text_size + 25.0
            };
            let y_offset = if screen_height() - position.y >= font_size * texts.len() as f32 {
                20.0
            } else {
                -font_size * texts.len() as f32
            };

            for (index, field) in &mut texts.iter_mut().enumerate() {
                draw_text(
                    field,
                    position.x + x_offset,
                    (position.y + y_offset) + index as f32 * 20.0,
                    font_size,
                    PINK,
                );
            }
        }
    }

    pub fn update(&mut self, delta_time: f64) {
        // Calculate velocity based on rotation and max speed
        let direction = vec2(self.rotation.cos(), -self.rotation.sin());

        // Prevent the spaceship from going faster than the max speed
        if self.get_speed() > self.get_max_speed() {
            self.speed = self.get_max_speed();
        }
        // Update position using the current speed and direction
        self.position += direction * self.speed * delta_time as f32;

        // Handle screen wrapping (loop the spaceship)
        if self.position.x < 0.0 {
            self.position.x = screen_width();
        } else if self.position.x > screen_width() {
            self.position.x = 0.0;
        }

        if self.position.y < 0.0 {
            self.position.y = screen_height();
        } else if self.position.y > screen_height() {
            self.position.y = 0.0;
        }

        if self.hom_cooldown > 0.0 {
            self.hom_cooldown = (self.hom_cooldown - delta_time).max(0.0);
        }
        if self.fire_cooldown > 0.0 {
            self.fire_cooldown = (self.fire_cooldown - delta_time).max(0.0);
        }
        if self.invulnerability > 0.0 {
            self.invulnerability = (self.invulnerability - delta_time).max(0.0);
        }
    }

    pub fn move_spaceship(&mut self, delta_time: f64, movement_type: bool) {
        let movement_direction = if movement_type { 1.0 } else { -1.0 };

        // The spaceship goes in the opposite direction faster
        let acceleration_factor = if self.get_speed().signum() == -movement_direction {
            3.0
        } else {
            1.0
        };
        let base_acceleration = 150.0;
        let acceleration =
            base_acceleration * movement_direction * acceleration_factor * delta_time as f32;

        // Accelerate if it would not go over the max speed attribute
        if (self.get_speed() + acceleration).abs() < self.get_max_speed() {
            self.speed += acceleration;
        // Cap the speed to the max speed attribute
        } else {
            self.speed = self.get_max_speed() * movement_direction;
        }
    }

    pub fn generate_positions_angles(
        &self,
        left_start: f32,
        left_end: f32,
        right_start: f32,
        right_end: f32,
    ) -> Vec<(Vec2, f32)> {
        let n = if self.get_missile_capacity() % 2 != 0 {
            self.get_missile_capacity() + 1
        } else {
            self.get_missile_capacity()
        };

        let center = self.get_position();
        let radius = self.get_special_radius();
        let rotation = self.get_rotation();

        let mut positions_angles = Vec::new();

        match n {
            0 => { /* Do nothing to prevent a crash */ }
            2 => {
                positions_angles = vec![
                    (
                        Vec2::new(
                            center.x + radius * (PI / 2.0 + rotation).cos(),
                            center.y - radius * (PI / 2.0 + rotation).sin(),
                        ),
                        PI / 2.0 + rotation,
                    ),
                    (
                        Vec2::new(
                            center.x + radius * (3.0 * PI / 2.0 + rotation).cos(),
                            center.y - radius * (3.0 * PI / 2.0 + rotation).sin(),
                        ),
                        3.0 * PI / 2.0 + rotation,
                    ),
                ];
            }
            4 => {
                positions_angles = vec![
                    (
                        Vec2::new(
                            center.x + radius * (2.0 * PI / 3.0 + rotation).cos(),
                            center.y - radius * (2.0 * PI / 3.0 + rotation).sin(),
                        ),
                        2.0 * PI / 3.0 + rotation,
                    ),
                    (
                        Vec2::new(
                            center.x + radius * (4.0 * PI / 3.0 + rotation).cos(),
                            center.y - radius * (4.0 * PI / 3.0 + rotation).sin(),
                        ),
                        4.0 * PI / 3.0 + rotation,
                    ),
                    (
                        Vec2::new(
                            center.x + radius * (PI / 3.0 + rotation).cos(),
                            center.y - radius * (PI / 3.0 + rotation).sin(),
                        ),
                        PI / 3.0 + rotation,
                    ),
                    (
                        Vec2::new(
                            center.x + radius * (5.0 * PI / 3.0 + rotation).cos(),
                            center.y - radius * (5.0 * PI / 3.0 + rotation).sin(),
                        ),
                        5.0 * PI / 3.0 + rotation,
                    ),
                ];
            }
            _ => {
                // Step size for both left and right sides
                let left_step = (left_end - left_start) / (n / 2 - 1) as f32;
                let right_step = (right_end - right_start) / (n / 2 - 1) as f32;

                // Generate positions for the left side (from left_start to left_end)
                for i in 0..(n / 2) {
                    let angle = left_start + i as f32 * left_step + rotation;
                    let x = center.x + radius * angle.cos();
                    let y = center.y - radius * angle.sin();
                    positions_angles.push((Vec2::new(x, y), angle));
                }

                // Generate positions for the right side (from right_start to right_end)
                for i in 0..(n / 2) {
                    let angle = right_start - i as f32 * right_step + rotation;
                    let x = center.x + radius * angle.cos();
                    let y = center.y - radius * angle.sin();
                    positions_angles.push((Vec2::new(x, y), angle));
                }
            }
        }

        positions_angles
    }

    pub fn stop(&mut self) {
        self.speed = 0.0;
    }

    pub fn get_speed(&self) -> f32 {
        self.speed
    }

    pub fn get_max_speed(&self) -> f32 {
        self.max_speed
    }

    pub fn get_rotation(&self) -> f32 {
        self.rotation
    }

    pub fn get_turn_rate(&self) -> f32 {
        self.turn_rate
    }

    pub fn get_missile_capacity(&self) -> u8 {
        self.missile_capacity
    }

    pub fn get_special_radius(&self) -> f32 {
        self.special_radius
    }

    pub fn get_size(&self) -> f32 {
        self.size
    }

    pub fn get_shield(&self) -> f32 {
        self.shield
    }

    pub fn get_invulnerability(&self) -> f64 {
        self.invulnerability
    }

    pub fn get_life(&self) -> bool {
        self.alive
    }

    pub fn get_homming_cooldown(&self) -> f64 {
        self.hom_cooldown
    }

    pub fn get_firing_cooldown(&self) -> f64 {
        self.fire_cooldown
    }

    pub fn modify_shield(&mut self, amount: f32) {
        self.shield += amount;
        if self.shield < 0.0 {
            self.shield = 0.0;
        }
    }

    #[allow(dead_code)]
    pub fn modify_invulnerability(&mut self, amount: f64) {
        self.invulnerability += amount;
    }

    pub fn modify_capacity(&mut self, delta: i8) {
        if delta >= 0 {
            // saturating_add to avoid overflow, then clamp to MAX-1
            self.missile_capacity =
                (self.missile_capacity.saturating_add(delta as u8)).min(u8::MAX - 1);
        } else {
            // saturating_sub ensures it never goes below 0
            self.missile_capacity = self.missile_capacity.saturating_sub((-delta) as u8);
        }
    }

    pub fn set_invulnerability(&mut self, amount: f64) {
        self.invulnerability = amount;
    }

    pub fn set_speed(&mut self, amount: f32) {
        self.speed = amount
    }

    pub fn set_rotation(&mut self, amount: f32) {
        self.rotation = amount
    }

    pub fn set_life(&mut self, state: bool) {
        self.alive = state
    }

    pub fn set_homming_cooldown(&mut self, amount: f64) {
        self.hom_cooldown = amount
    }

    pub fn set_firing_cooldown(&mut self, amount: f64) {
        self.fire_cooldown = amount
    }
}
