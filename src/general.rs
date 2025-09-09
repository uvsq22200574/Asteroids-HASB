use crate::asteroid::Asteroid;
use crate::helpers::{apply_changes, Change, Entity, TEXTURE_SET};
use crate::missile::Missile;
use crate::spaceship::Spaceship;
use crate::LifetimedText;

use macroquad::color::WHITE;
use macroquad::prelude::{
    draw_texture_ex, screen_height, screen_width, DrawTextureParams, Texture2D, Vec2,
};

use std::path::PathBuf;

pub const TICKS: f64 = 1.0 / 60.0;

pub struct Gamestate {
    pub delta_time: f64,
    pub accumulator: f64,
    pub simulation_speed: f64,
    pub fps: u32,
    pub fps_cooldown: f64,
    pub debug: bool,
    pub loop_number: u128,
    pub input: Vec<String>,

    pub asteroids: Vec<Asteroid>,
    pub asteroids_children: u8,
    pub missiles: Vec<Missile>,
    pub spaceship: Spaceship,
    pub asteroid_limit: u8,
    pub number_of_asteroids: u32,
    pub score: [u128; 2],
    pub multipliers: Vec<u8>,

    pub asteroid_changes: Vec<Change<Asteroid>>,
    pub missile_changes: Vec<Change<Missile>>,
    pub text_changes: Vec<Change<LifetimedText>>,

    pub menu: Vec<String>,
    pub win: bool,
    pub over: bool,
    pub exit: bool,
    pub texts: Vec<LifetimedText>,
}

// The multipliers contains the size of the asteroid as the index-1
impl Gamestate {
    pub fn new() -> Gamestate {
        Gamestate {
            delta_time: 0.0,
            accumulator: 0.0,
            simulation_speed: 0.0,
            fps: 0,
            fps_cooldown: 0.0,
            debug: false,
            loop_number: 0,
            input: Vec::new(),

            asteroids: Vec::new(),
            asteroids_children: 2,
            missiles: Vec::new(),
            spaceship: Spaceship::new(),
            asteroid_limit: 26,
            number_of_asteroids: 0,
            score: [0, 0],
            multipliers: vec![3, 2, 1],

            asteroid_changes: Vec::new(),
            missile_changes: Vec::new(),
            text_changes: Vec::new(),

            menu: vec![String::from("Start")],
            win: false,
            over: false,
            exit: false,
            texts: Vec::new(),
        }
    }

    /// Reset the gamestate to a playable environment
    pub fn reset(&mut self) {
        self.win = false;
        self.over = false;
        self.asteroids.clear();
        self.missiles.clear();
        self.spaceship = Spaceship::new();
        self.texts = Vec::new();
        self.menu.pop();
        for _ in 1..=20 {
            self.asteroids.push(Asteroid::new_default());
        }
        self.number_of_asteroids = self.asteroids.len() as u32;
        let mults = &self.multipliers;
        self.score = [
            0,
            self.get_max_score(100, &mults, self.asteroids_children, self.debug)[3],
        ];
    }

    /// Get a texture by PathBuf key. Falls back to "missing.png" if not found.
    pub fn get_texture(&self, file: &PathBuf) -> &Texture2D {
        if let Some(texture) = TEXTURE_SET.get(file) {
            texture
        } else {
            eprintln!("[WARN] Texture {:?} not found, using default.", file);
            TEXTURE_SET
                .get(&PathBuf::from("missing.png"))
                .expect("Default texture missing!")
        }
    }

    pub fn update_fps(&mut self) {
        if macroquad::prelude::get_time() - self.fps_cooldown >= 1.0 / 4.0 {
            self.fps = macroquad::time::get_fps() as u32;
            self.fps_cooldown = macroquad::prelude::get_time();
        }
    }

    pub fn update_spaceship(&mut self) {
        self.spaceship.update(self.delta_time);
    }

    pub fn update_missiles(&mut self) {
        for missile in &mut self.missiles {
            missile.update(&self.asteroids, self.delta_time);
        }
    }

    pub fn update_asteroids(&mut self) {
        for asteroid in &mut self.asteroids {
            asteroid.update(self.delta_time);
        }
    }

    pub fn update_scores(&mut self) {
        // Floating texts
        for text in &mut self.texts {
            text.update(self.delta_time);
        }
    }

    pub fn update_simulation_speed(&mut self) {
        if self.menu.is_empty() {
            self.simulation_speed = 1.0;
        } else if !self.debug {
            self.simulation_speed = 0.0;
        }

        // Pause state when there is a menu
        if !self.menu.is_empty() && !(self.get_last_menu_item() == "Start" && self.debug) {
            self.simulation_speed = 0.0;
        }
        // Slow motion when Game over
        if !self.spaceship.get_life() && self.simulation_speed == 1.0 && !self.debug {
            self.simulation_speed = 0.05;
        }
    }

    pub fn update_ending(&mut self) {
        // Ending Conditions
        if !self.spaceship.get_life() && self.simulation_speed > 0.0 && !self.debug {
            self.simulation_speed = 0.1;
            if self.number_of_asteroids <= 0 {
                self.over = true;
                if self.menu.is_empty() {
                    self.menu.push(String::from("Start"));
                }
            }
        }

        if self.spaceship.get_life() && self.simulation_speed > 0.0 && self.number_of_asteroids <= 0
        {
            self.win = true;
            if self.menu.is_empty() && !self.debug {
                self.menu.push(String::from("Start"));
            }
        }
    }

    /// Makes the updates of the simulation so things moves and interact
    pub fn update_all(&mut self) {
        // Update every element
        self.update_fps();
        self.update_spaceship();
        self.update_missiles();
        self.update_asteroids();
        self.update_scores();
        self.update_simulation_speed();

        // Remove destroyed objects
        apply_changes(&mut self.asteroids, &mut self.asteroid_changes);
        apply_changes(&mut self.missiles, &mut self.missile_changes);
        apply_changes(&mut self.texts, &mut self.text_changes);

        self.number_of_asteroids = self.asteroids.len() as u32;

        self.update_ending();
    }

    pub fn draw_all(&mut self) {
        // Background
        if !self.debug {
            draw_texture_ex(
                self.get_texture(&PathBuf::from("background2.png")),
                0.0,
                0.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(screen_width(), screen_height())),
                    ..Default::default()
                },
            );
        }

        // Draw asteroids
        for asteroid in &self.asteroids {
            asteroid.draw_self(self.debug);
        }

        // Draw spaceship
        if self.spaceship.get_life() {
            self.spaceship.draw(25.0, self.delta_time, self.debug);
        }

        // Draw missiles
        for missile in &self.missiles {
            missile.draw(self.debug);
        }

        // Draw the score obtained
        for text_bubble in &self.texts {
            text_bubble.display();
        }
    }

    /// Returns an array with the first elements being the distribution of sizes
    /// [10.0, 20.0, 30.0] and the last element being the total score.
    pub fn get_max_score(
        &self,
        base_score: u128,
        multipliers: &[u8], // expected to have length 3
        children_count: u8,
        print: bool,
    ) -> [u128; 4] {
        let mut result: [u128; 4] = [0; 4]; // [size10, size20, size30, total]

        // Recursive helper function to count asteroids by size
        fn accumulate_size(result: &mut [u128; 4], size: f32, children_count: u8) {
            if size < Asteroid::SCALE {
                return; // no smaller asteroids
            }

            // Map size to index: 10 -> 0, 20 -> 1, 30 -> 2
            let index = ((size / Asteroid::SCALE).round() as usize) - 1;
            if index < 3 {
                result[index] += 1;
            }

            // Recursively add children
            for _ in 0..children_count {
                accumulate_size(result, size - Asteroid::SCALE, children_count);
            }
        }

        // Process all asteroids in the game state
        for asteroid in &self.asteroids {
            accumulate_size(&mut result, asteroid.get_size(), children_count);
        }

        // Compute total score
        let mut total_score: u128 = 0;
        for (index, &multiplier) in multipliers.iter().enumerate().take(3) {
            let computed_score = result[index] * multiplier as u128 * base_score;
            if print {
                println!(
                    "{}x{}x{}={}",
                    base_score, multiplier, result[index], computed_score
                );
            }
            total_score += computed_score;
        }
        result[3] = total_score;

        result
    }

    // === Helper Functions ===

    /// Will return the last current menu
    pub fn get_last_menu_item(&self) -> &str {
        self.menu.last().map(|s| s.as_str()).unwrap_or("")
    }

    /// Will summon a missile from the spaceship
    pub fn summon_missile(&mut self, is_homing: bool) {
        if is_homing {
            let capacity = self.spaceship.get_missile_capacity() as usize;
            let positions = self.spaceship.generate_positions_angles(
                std::f32::consts::PI / 2.0 + 0.2,
                3.0 * std::f32::consts::PI / 2.0,
                std::f32::consts::PI / 2.0,
                3.0 * std::f32::consts::PI / 2.0 - 0.2,
            );

            for idx in 0..capacity {
                self.missiles.push(Missile::new(
                    positions[idx].0,
                    200.0,
                    positions[idx].1,
                    is_homing,
                    Vec2::from_array([-100.0; 2]),
                ));
            }
        } else {
            self.missiles.push(Missile::new(
                self.spaceship.get_position(),
                self.spaceship.get_max_speed(),
                self.spaceship.get_rotation(),
                is_homing,
                Vec2::from_array([-100.0; 2]),
            ));
        }
    }

    // === DEBUG COMMANDS ===
    pub fn split_all_asteroids(&mut self) {
        for asteroid in &mut self.asteroids {
            asteroid.split(true, self.asteroids_children, &mut self.asteroid_changes);
        }
    }

    pub fn create_debug_asteroid(&mut self) {
        let asteroid_position = self.spaceship.position_in_front_with_rotation(500.0, 0.0);

        let asteroid = Asteroid::new(
            Some(asteroid_position),
            Some(0.0),                   // stationary
            Some(3.0 * Asteroid::SCALE), // size
            None,
            None,
            None,
            None,
            None,
        );

        self.asteroid_changes.push(Change::Add(asteroid));
    }
}
