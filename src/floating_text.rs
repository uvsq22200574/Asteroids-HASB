use crate::helpers::generate_uid;
use crate::import_entity;
use macroquad::prelude::Color;
// Call the macro
import_entity!(LifetimedText);

#[derive(PartialEq, Clone)]
pub struct LifetimedText {
    id: u64,
    lifetime: f64,
    position: Vec2,
    rotation: f32,
    text: String,
    size: f32,
    color: Color,
    speed: f32,
}

impl LifetimedText {
    pub fn new(
        lifetime: f64,
        position: Vec2,
        rotation: f32,
        text: String,
        font_size: f32,
        color: Color,
        speed: f32,
    ) -> LifetimedText {
        return LifetimedText {
            id: generate_uid(),
            lifetime: lifetime,
            position: position,
            rotation: rotation,
            text: text,
            size: font_size,
            color: color,
            speed: speed,
        };
    }

    pub fn display(&self) {
        draw_text(
            &self.text.to_string(),
            self.position.x,
            self.position.y,
            self.size,
            self.color,
        );
    }

    pub fn update(&mut self, deltatime: f64) {
        self.position.y += self.speed * deltatime as f32;
        self.lifetime -= deltatime;
    }

    pub fn get_lifetime(&self) -> f64 {
        self.lifetime
    }
}
