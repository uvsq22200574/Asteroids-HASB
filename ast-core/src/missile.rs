use ast_lib::generate_uid;
use entity_derive::Entity;
use ast_lib::CosmicEntity;

use macroquad::prelude::{
    draw_circle, draw_text, measure_text, screen_dpi_scale, screen_height, screen_width, vec2, Vec2, GOLD, GRAY, MAGENTA,
    RED,
};
use std::f32::consts::PI;

#[derive(PartialEq, Clone, Entity)]
pub struct Missile {
    id: u64,
    position: Vec2,
    speed: f32,
    rotation: f32,
    lifetime: f64,
    size: f32,
    turn_rate: f32,
    acceleration: f32,
    homing: bool,
    target: Vec2,
}

#[allow(unused)]
impl Missile {
    /// Create a missile projectile
    pub fn new(position: Vec2, speed: f32, rotation: f32, homing: bool, target: Vec2) -> Self {
        Self {
            id: generate_uid(),
            position,
            speed: speed.abs(),
            rotation,
            lifetime: 20.0,
            size: 4.0,
            turn_rate: 7.5,
            acceleration: 200.0,
            homing,
            target,
        }
    }

    pub fn get_speed(&self) -> f32 {
        self.speed
    }

    pub fn get_rotation(&self) -> f32 {
        self.rotation
    }

    pub fn get_lifetime(&self) -> f64 {
        self.lifetime
    }

    pub fn get_size(&self) -> f32 {
        self.size
    }

    pub fn get_turn_rate(&self) -> f32 {
        self.turn_rate
    }

    pub fn get_acceleration(&self) -> f32 {
        self.acceleration
    }

    pub fn is_homing(&self) -> bool {
        self.homing
    }

    pub fn get_target(&self) -> Vec2 {
        self.target
    }

    /// Change the direction to target a position
    pub fn target_object(&mut self, target_pos: Vec2, delta_time: f64) {
        let direction_to_target: Vec2 = target_pos - self.position;
        let target_angle = -direction_to_target.y.atan2(direction_to_target.x);

        let mut angle_diff = target_angle - self.rotation;

        // Normalize to -PI..PI
        if angle_diff > PI {
            angle_diff -= 2.0 * PI;
        } else if angle_diff < -PI {
            angle_diff += 2.0 * PI;
        }

        let max_turn = self.turn_rate * delta_time as f32;

        if angle_diff.abs() > max_turn {
            if angle_diff > 0.0 {
                self.rotation += max_turn;
            } else {
                self.rotation -= max_turn;
            }
        } else {
            self.rotation = target_angle;
        }
    }

    /// Update missile state
    pub fn update(&mut self, potential_targets: &Vec<crate::asteroid::Asteroid>, delta_time: f64) {
        if self.homing {
            let nearest_target = self.find_nearest(potential_targets);
            self.speed += self.acceleration * delta_time as f32;

            if self.turn_rate > 1.0 {
                self.turn_rate -= 3.5 * delta_time as f32;
            }
            if self.lifetime > 0.0 {
                self.lifetime -= delta_time;

                if self.homing {
                    self.target = nearest_target.unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                    self.target_object(self.target, delta_time);
                }
            } else {
                self.turn_rate = 0.0;
            }
        }
        self.rotation %= PI * 2.0;
        self.position +=
            vec2(self.rotation.cos(), -self.rotation.sin()) * (self.speed) * delta_time as f32;

        if self.position.x < 0.0
            || self.position.x > screen_width()
            || self.position.y < 0.0
            || self.position.y > screen_height()
        {
            self.size = 0.0;
        }
    }

    /// Draw the missile
    pub fn draw(&self, debug: bool) {
        if self.homing {
            if self.lifetime > 0.0 {
                draw_circle(self.position.x, self.position.y, self.size * 1.25, MAGENTA);
            } else {
                draw_circle(self.position.x, self.position.y, self.size, GRAY);
            }
        } else {
            draw_circle(self.position.x, self.position.y, self.size, RED);
        }

        if debug {
            let font_size = 15.0;
            let position = self.position;
            let mut texts = Vec::from([
                format!("x:{:.2} y:{:.2}", position.x, position.y),
                format!("Lifetime:{:.2}s", self.lifetime),
                format!("Speed:{:.2}px/s", self.speed),
                format!("Tx:{:.2} Ty:{:.2}", self.target.x, self.target.y),
                format!("Rot: {:.2} deg", self.rotation * 180.0 / PI),
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
                5.0
            } else {
                -text_size + 5.0
            };
            let y_offset = if screen_height() - position.y >= font_size * texts.len() as f32 {
                10.0
            } else {
                -font_size * texts.len() as f32
            };

            for (index, field) in &mut texts.iter_mut().enumerate() {
                draw_text(
                    field,
                    position.x + x_offset,
                    (position.y + y_offset) + index as f32 * 20.0,
                    font_size,
                    GOLD,
                );
            }
        }
    }
}
