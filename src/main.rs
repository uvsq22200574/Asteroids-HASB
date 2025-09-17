// Load the modules
mod asteroid;
mod floating_text;
mod general;
mod helpers;
mod key_bindings;
mod menus;
mod missile;
mod spaceship;

use ::rand::{thread_rng, Rng};
use macroquad::prelude::{
    get_time, next_frame, screen_height, screen_width, vec2, Vec2, GOLD, GREEN, MAGENTA, WHITE,
};

// Keybindings
use floating_text::LifetimedText;
use key_bindings::KeyBindings;

use crate::{
    asteroid::Asteroid,
    helpers::{apply_changes, Change, Entity},
};
use general::{Gamestate, TICKS};

fn window_conf() -> macroquad::window::Conf {
    macroquad::window::Conf {
        window_title: "Asteroids".to_owned(),
        fullscreen: true,
        window_resizable: true,
        window_height: 1440,
        window_width: 2560,
        ..Default::default()
    }
}

/*
For reference visit https://macroquad.rs/examples/
Altough it's outdated and vastly different
*/

/* TO REWRITE */

/// The main entry point of the Asteroids game.
///
/// This function initializes the game environment, including window parameters, textures, and game state.
/// It then enters the game loop, which:
/// - Handles player input.
/// - Updates the game state (e.g., physics, collisions, and game logic).
/// - Renders the game visuals, including the background, asteroids, spaceship, and missiles.
///
/// The game loop is designed to be frame-rate independent using a delta-time (DT) system.
/// This ensures smooth animations and consistent behavior, regardless of the frame rate.
///
/// # Features
/// - **Initialization**: Sets up the game window and loads necessary textures.
/// - **Game Loop**:
///   - Handles input from the player (e.g., spaceship movement, firing missiles).
///   - Updates the positions, rotations, and states of game objects.
///   - Checks for and resolves collisions (e.g., between asteroids, missiles, and the spaceship).
///   - Displays the current game state, including debug information if enabled.
/// - **Game State Management**:
///   - Ends the game when the player loses all lives or destroys all asteroids.
///   - Manages transitions to the main menu or victory screen.
///
/// # Delta-Time System
/// The delta-time (DT) system ensures physics calculations and movements are time-based rather than
/// frame-based. This provides consistent gameplay across varying system performance.
///
/// # Window Initialization
/// - The game starts in fullscreen mode.
/// - Waits until the screen dimensions are updated from the default `800x600`.
///
/// Once launched, use the appropriate input controls to play.
///
/// # Notes
/// - This function runs asynchronously due to the use of the `macroquad` library.
/// - Ensure all textures are placed in the correct directory to avoid runtime errors.
///
/// # See Also
/// - [`Gamestate`](./gamestate.rs): The core structure that tracks the game's state.
#[macroquad::main(window_conf)]

async fn main() {
    let mut gamestate = Gamestate::new();

    let mut previous_time = 0.0;
    let mut end_cooldown = get_time();
    let mut rng = thread_rng();

    // Initialize keybindings
    let keybindings: KeyBindings = match KeyBindings::load("keybindings.json") {
        Ok(kb) => kb,
        Err(_) => {
            println!("Couldn't find the keybinds file");
            key_bindings::default_keybindings()
        }
    };
    println!("Don't forget that keybinds do not update automatically if the file is there !");
    keybindings.start_listening();

    loop {
        // Delta-time
        gamestate.delta_time = (get_time() - previous_time) * gamestate.simulation_speed;
        previous_time = get_time();

        // Bounds
        let bounds = Vec2 {
            x: screen_width(),
            y: screen_height(),
        };

        // Handle input
        gamestate.input = keybindings.get_held_keys();

        // Update simulation
        gamestate.accumulator += gamestate.delta_time;
        while gamestate.accumulator >= TICKS {
            gamestate.loop_number += 1;

            // Discard missiles that are out of bounds
            for missile in &gamestate.missiles {
                if missile.is_out_of_bounds(&bounds) {
                    gamestate
                        .missile_changes
                        .push(Change::Remove(missile.get_id()));
                    continue;
                }
            }

            apply_changes(&mut gamestate.missiles, &mut gamestate.missile_changes);

            for asteroid in &mut gamestate.asteroids {
                if asteroid.get_size() == 0.0 {
                    gamestate
                        .asteroid_changes
                        .push(Change::Remove(asteroid.get_id()));
                }
                // Check the collision between the SPACESHIP and ASTEROIDS
                let spaceship_collision = asteroid.collides_with(&gamestate.spaceship);

                if gamestate.spaceship.get_life()
                    && gamestate.spaceship.get_invulnerability() <= 0.0
                    && spaceship_collision
                {
                    gamestate
                        .asteroid_changes
                        .push(Change::Remove(asteroid.get_id()));
                    asteroid.split(
                        (gamestate.number_of_asteroids + gamestate.asteroids_children as u32)
                            < gamestate.asteroid_limit.into(),
                        gamestate.asteroids_children,
                        &mut gamestate.asteroid_changes,
                    );

                    gamestate.spaceship.modify_shield(
                        -(5.0 / 3.0 * (asteroid.get_size() / Asteroid::SCALE + 1.0).powf(2.0)),
                    );

                    gamestate.spaceship.set_invulnerability(0.4);
                    gamestate
                        .spaceship
                        .set_speed(gamestate.spaceship.get_speed() * 0.25);
                    gamestate
                        .spaceship
                        .add_rotation(rng.gen_range(1.0..std::f32::consts::PI));

                    if gamestate.spaceship.get_shield() <= 0.0 {
                        gamestate.spaceship.set_life(false);
                    }
                }

                // Missile collisions
                for missile in &gamestate.missiles {
                    let collision = asteroid.collides_with(missile);
                    if collision {
                        gamestate
                            .missile_changes
                            .push(Change::Remove(missile.get_id()));
                        let already_removed = gamestate
                            .asteroid_changes
                            .iter()
                            .any(|c| matches!(c, Change::Remove(a) if *a == asteroid.get_id()));

                        if !already_removed {
                            asteroid.split(
                                (gamestate.number_of_asteroids
                                    + gamestate.asteroids_children as u32)
                                    < gamestate.asteroid_limit.into(),
                                gamestate.asteroids_children,
                                &mut gamestate.asteroid_changes,
                            );

                            let score = asteroid
                                .grant_score(&mut gamestate.score[0], &gamestate.multipliers);

                            gamestate.text_changes.push(Change::Add(LifetimedText::new(
                                match score {
                                    100 => 1.0,
                                    200 => 2.0,
                                    300 => 2.5,
                                    _ => 1.0,
                                },
                                missile.get_position()
                                    + vec2(
                                        rng.gen_range(-50.0..=50.0), // Random X offset
                                        rng.gen_range(-100.0..=100.0),
                                    ), // Random Y offset,
                                0.0,
                                score.to_string(),
                                match score {
                                    100 => 30.0,
                                    200 => 35.0,
                                    300 => 45.0,
                                    _ => 30.0,
                                },
                                match score {
                                    100 => GREEN,
                                    200 => GOLD,
                                    300 => MAGENTA,
                                    _ => WHITE,
                                },
                                -30.0,
                            )));
                        }
                    }
                }
            }

            for text_bubble in &gamestate.texts {
                if text_bubble.get_lifetime() <= 0.0 {
                    gamestate
                        .text_changes
                        .push(Change::Remove(text_bubble.get_id()));
                }
            }

            // Remove asteroids when the ship is destroyed
            if !gamestate.spaceship.get_life()
                && get_time() - end_cooldown >= 0.5
                && gamestate.simulation_speed > 0.0
            {
                let mut rng = thread_rng();
                for asteroid in &mut gamestate.asteroids {
                    if rng.gen_range(0..=100) <= 50 {
                        asteroid.split(
                            (gamestate.number_of_asteroids + gamestate.asteroids_children as u32)
                                < gamestate.asteroid_limit.into(),
                            gamestate.asteroids_children,
                            &mut gamestate.asteroid_changes,
                        );
                    }
                }
                end_cooldown = get_time();
            }

            gamestate.accumulator -= TICKS;
        }

        gamestate.update_all();
        gamestate.draw_all();

        key_bindings::handle_input(&mut gamestate, &keybindings);

        // Menu and UI
        menus::draw_simulation(&gamestate);
        let action = menus::menu_draw(&mut gamestate, bounds.x, bounds.y);
        match action.as_str() {
            "Exit" => break,
            "Clear" => {
                gamestate.asteroids.clear();
                gamestate.score = [0, 0];
            }
            "Split All" => {
                gamestate.split_all_asteroids();
            }
            "Summon Asteroid" => {
                gamestate.create_debug_asteroid();
            }
            _ => (),
        }
        if gamestate.exit {
            println!("Exiting...");
            break;
        }

        next_frame().await;
    }

    // Save keybindings on exit
    if let Err(e) = keybindings.save("keybindings.json") {
        eprintln!("Failed to save keybindings: {:?}", e);
    }
}
