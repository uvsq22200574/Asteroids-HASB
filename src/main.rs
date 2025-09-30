use ::rand::{Rng, thread_rng};
use ast_core::{
    asteroid::Asteroid,
    floating_text::LifetimedText,
    gamestate::{Gamestate, TICKS},
    key_bindings, menus,
};
use ast_lib::{Change, CosmicEntity, apply_changes};
use macroquad::prelude::{
    GOLD, GREEN, MAGENTA, Vec2, WHITE, get_time, next_frame, screen_height, screen_width, vec2,
};

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

/// Entry point of the Asteroids simulation/game.
///
/// This function sets up the application window (via the `#[macroquad::main]` attribute),
/// initializes the game state, loads user keybindings, and then runs the main game loop.
/// The loop is asynchronous (`async fn main`) to allow `macroquad`’s frame scheduling.
///
/// # Responsibilities
///
/// - **Initialization**
///   - Creates a new [`Gamestate`] instance.
///   - Loads keybindings from `keybindings.json`, falling back to defaults if the file is missing.
///   - Starts listening for key events.
///   - Sets up random number generation and time tracking.
///
/// - **Game loop**
///   Runs continuously until the player exits. Each iteration:
///
///   1. Computes delta time and updates the simulation accumulator.
///   2. Register input by recording currently held keys.
///   3. Performs fixed-timestep updates while the accumulator exceeds the tick interval:
///      - Removes missiles outside the screen bounds.
///      - Applies queued changes to the missile list.
///      - Updates asteroids:
///        - Removes destroyed asteroids.
///        - Detects and resolves collisions with the spaceship.
///        - Detects and resolves collisions with missiles, updating score and spawning text.
///      - Removes expired text popups.
///      - Randomly discards asteroids (cooldown-based).
///   4. Renders the current state (`update_all`, `draw_all`).
///   5. Processes input handling via [`key_bindings::handle_input`].
///   6. Draws simulation menus and executes menu-driven actions such as:
///      - Exit the game
///      - Clear all asteroids and reset score
///      - Split all asteroids
///      - Spawn a debug asteroid
///
/// - **Exit**
///   - Exits the loop when the player chooses "Exit" in the menu or when `gamestate.exit` is set.
///   - Saves the current keybindings back to `keybindings.json`.
///
/// # Notes
/// - The simulation speed is managed using `delta_time` and `accumulator`
///   to ensure fixed-timestep updates (`TICKS` constant).
/// - Collisions are resolved deterministically inside the asteroid update loop.
/// - Randomness (`thread_rng`) is used for asteroid splitting, collision knockback,
///   and text placement offsets.
/// - UI and menus are drawn each frame after simulation updates.
///
/// # Panics
/// - This function does not explicitly panic, but may panic indirectly if
///   asset loading, drawing, or file I/O in dependent modules fails.
///
/// # Errors
/// - Keybindings save errors are caught at the end of execution and logged to stderr.
#[macroquad::main(window_conf)]

async fn main() {
    let mut gamestate = Gamestate::new();

    let mut previous_time = 0.0;
    let mut end_cooldown = get_time();
    let mut rng = thread_rng();

    // Initialize keybindings
    let keybindings: key_bindings::KeyBindings =
        match key_bindings::KeyBindings::load("keybindings.json") {
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

        // Register input
        gamestate.input = keybindings.get_held_keys();

        // Update simulation
        gamestate.accumulator += gamestate.delta_time;
        while gamestate.accumulator >= TICKS {
            gamestate.loop_number += 1;

            gamestate.discard_out_of_bounds_missiles(&bounds);

            apply_changes(&mut gamestate.missiles, &mut gamestate.missile_changes);

            // Main part of the loop
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

            gamestate.discard_texts();

            gamestate.discard_asteroids_random(get_time(), &mut end_cooldown, 10);

            gamestate.accumulator -= TICKS;
        }

        gamestate.update_all();
        gamestate.draw_all();

        // Apply keybindings actions
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
