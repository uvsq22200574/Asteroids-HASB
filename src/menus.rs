use chrono::Local;
use macroquad::prelude::*;
use std::env;

use crate::general::Gamestate;

fn button(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text: &str,
    font_size: f32,
    text_color: Color,
) -> bool {
    // Get mouse position
    let mouse_x = mouse_position().0;
    let mouse_y = mouse_position().1;

    // Check if mouse is over the button
    let is_hovered = mouse_x >= x && mouse_x <= x + width && mouse_y >= y && mouse_y <= y + height;

    // Draw the button (with hover effect)
    if is_hovered {
        draw_rectangle(x, y, width, height, Color::from_rgba(0, 255, 128, 255));
    // Hover color
    } else {
        draw_rectangle(x, y, width, height, Color::from_rgba(0, 255, 196, 255));
        // Normal color
    }

    // Draw the text on top of the button
    draw_text(
        text,
        x + (width / 2.0) - (measure_text(text, None, font_size as u16, 1.0).width / 2.0),
        y + (height / 2.0) + (measure_text(text, None, font_size as u16, 1.0).height / 2.0) - 5.0,
        font_size,
        text_color,
    );

    // Return whether the button was clicked
    is_hovered && is_mouse_button_pressed(MouseButton::Left)
}

pub fn menu_draw(gamestate: &mut Gamestate, screen_width: f32, screen_height: f32) -> String {
    let current_menu = gamestate.get_last_menu_item();

    // During Gameplay
    if current_menu != "Start" && !gamestate.menu.is_empty() {
        // Darken first
        draw_rectangle(
            0.0,
            0.0,
            screen_width,
            screen_height,
            Color::from_rgba(0, 0, 0, 128),
        );
        // White transparent background
        draw_rectangle(
            0.0,
            0.0,
            screen_width,
            screen_height,
            Color::from_rgba(255, 255, 255, 32),
        );
        draw_text(
            "Pause Menu",
            (screen_width - measure_text("Pause Menu", None, 80, 1.0).width) / 2.0,
            100.0,
            80.0,
            WHITE,
        );
    }

    // Main menu
    if current_menu == "Main" {
        if button(
            screen_width / 2.0 - 0.35 * screen_width,
            screen_height * 0.25 + 0.1 * screen_height * 1.0,
            0.35 * screen_width * 2.0,
            0.05 * screen_height,
            "Quit",
            60.0,
            RED,
        ) {
            return String::from("Exit");
        }
        #[cfg(debug_assertions)]
        if button(
            screen_width / 2.0 - 0.35 * screen_width,
            screen_height * 0.25 + 0.1 * screen_height * 2.0,
            0.35 * screen_width * 2.0,
            0.05 * screen_height,
            "Clear",
            60.0,
            ORANGE,
        ) {
            return String::from("Clear");
        }
        #[cfg(debug_assertions)]
        if button(
            screen_width / 2.0 - 0.35 * screen_width,
            screen_height * 0.25 + 0.1 * screen_height * 3.0,
            0.35 * screen_width * 2.0,
            0.05 * screen_height,
            "Split all",
            60.0,
            ORANGE,
        ) {
            return String::from("Split All");
        }

        if button(
            screen_width / 2.0 - 0.35 * screen_width,
            screen_height * 0.25 + 0.1 * screen_height * 4.0,
            0.35 * screen_width * 2.0,
            0.05 * screen_height,
            "Get Hardware",
            60.0,
            PURPLE,
        ) {
            gamestate.menu.push(String::from("Hardware"));
        }

        #[cfg(debug_assertions)]
        if button(
            screen_width / 2.0 - 0.35 * screen_width,
            screen_height * 0.25 + 0.1 * screen_height * 5.0,
            0.35 * screen_width * 2.0,
            0.05 * screen_height,
            "Spawn Debug Asteroids",
            60.0,
            PURPLE,
        ) {
            return String::from("Summon Asteroid");
        }
    }
    // Hardware menu
    else if current_menu == "Hardware" {
        let screen_width_start = 0.25;
        draw_text(
            &(format!("Operating System: {}", env::consts::OS.to_uppercase())),
            screen_width * screen_width_start,
            screen_height * 0.25 + 1.0 * 50.0,
            48.0,
            GREEN,
        );
        draw_text(
            &(format!("Screen Dimensions: {}X{}", screen_width, screen_height)),
            screen_width * screen_width_start,
            screen_height * 0.25 + 2.0 * 50.0,
            48.0,
            GREEN,
        );
        draw_text(
            &(format!("DPI scaling: {:.0}%", screen_dpi_scale() * 100.0)),
            screen_width * screen_width_start,
            screen_height * 0.25 + 3.0 * 50.0,
            48.0,
            GREEN,
        );
        draw_text(
            "Note that on certain OSes the fullscreen dimensions take into account the DPI",
            screen_width * screen_width_start,
            screen_height * 0.25 + 3.75 * 50.0,
            32.0,
            BEIGE,
        );
        draw_text(
            &format!("FPS: {}", gamestate.fps),
            screen_width * screen_width_start,
            screen_height * 0.25 + 5.0 * 50.0,
            48.0,
            GREEN,
        );
        draw_text(
            "The FPS counter is updated 4 times a second and may register with innacurracy",
            screen_width * screen_width_start,
            screen_height * 0.25 + 5.75 * 50.0,
            32.0,
            BEIGE,
        );
        draw_text(
            &"Keys Down:",
            screen_width * screen_width_start,
            screen_height * 0.25 + 6.5 * 50.0,
            48.0,
            GREEN,
        );
        draw_text(
            &format!("{:?}", gamestate.input),
            screen_width * screen_width_start,
            screen_height * 0.25 + 7.25 * 50.0,
            32.0,
            BEIGE,
        );
    }
    // Start Menu
    else if current_menu == "Start" && !gamestate.debug {
        let score = gamestate.score;

        clear_background(BLACK);
        draw_text(
            "ASTEROIDS",
            screen_width / 2.0
                - measure_text("ASTEROIDS", None, 40, screen_dpi_scale()).width / 2.0,
            screen_height / 2.0 - 50.0,
            40.0,
            WHITE,
        );
        draw_text(
            "Press ENTER to start",
            screen_width / 2.0
                - measure_text("Press ENTER to start", None, 30, screen_dpi_scale()).width / 2.0,
            screen_height / 2.0,
            30.0,
            GRAY,
        );
        draw_text(
            "Press Esc to quit",
            screen_width / 2.0
                - measure_text("Press Esc to quit", None, 30, screen_dpi_scale()).width / 2.0,
            screen_height / 2.0 + 50.0,
            30.0,
            GRAY,
        );

        if gamestate.over {
            draw_text(
                "GAME OVER",
                screen_width / 2.0
                    - measure_text("GAME OVER", None, 60, screen_dpi_scale()).width / 2.0,
                screen_height / 2.0 - 150.0,
                60.0,
                RED,
            );
            draw_text(
                &format!("Score: {}/{}", score[0], score[1]),
                screen_width / 2.0
                    - measure_text(
                        &format!("Score: {}/{}", gamestate.score[0], gamestate.score[1]),
                        None,
                        30,
                        screen_dpi_scale(),
                    )
                    .width
                        / 2.0,
                screen_height / 2.0 - 125.0,
                30.0,
                RED,
            );
        } else if gamestate.win {
            draw_text(
                "YOU WIN",
                screen_width / 2.0
                    - measure_text("YOU WIN", None, 60, screen_dpi_scale()).width / 2.0,
                screen_height / 2.0 - 150.0,
                60.0,
                GREEN,
            );
            draw_text(
                &format!("Score: {}/{}", score[0], score[1]),
                screen_width / 2.0
                    - measure_text(
                        &format!("Score: {}/{}", score[0], score[1]),
                        None,
                        30,
                        screen_dpi_scale(),
                    )
                    .width
                        / 2.0,
                screen_height / 2.0 - 125.0,
                30.0,
                GREEN,
            );
        }
    }
    return String::from("");
}

pub fn draw_simulation(gamestate: &Gamestate) {
    let mut positions = [50.0, 100.0, 150.0, 0.0, 0.0, 25.0];
    if gamestate.debug {
        positions = [50.0, 100.0, 150.0, 200.0, 250.0, 25.0];
        draw_text(
            &(format!("Loop:{}", gamestate.loop_number)),
            10.0,
            positions[3],
            48.0,
            RED,
        );
        draw_text(
            &(format!("Time:{}", Local::now().format("%H:%M:%S"))),
            10.0,
            positions[4],
            48.0,
            YELLOW,
        );
        draw_text(
            &(format!("Speed factor:{}x", gamestate.simulation_speed)),
            (screen_width()
                - measure_text(
                    &format!("Speed factor:{}x", gamestate.simulation_speed),
                    None,
                    36,
                    screen_dpi_scale(),
                )
                .width)
                / 2.0,
            positions[5],
            36.0,
            GOLD,
        );
    }
    // Always draw
    draw_text(
        &(format!("FPS:{}", gamestate.fps)),
        10.0,
        positions[0],
        48.0,
        GREEN,
    );
    draw_text(
        &(format!("Astéroides:{}", gamestate.number_of_asteroids)),
        10.0,
        positions[1],
        48.0,
        BLUE,
    );
    draw_text(
        &(format!("Missiles:{}", gamestate.missiles.len())),
        10.0,
        positions[2],
        48.0,
        BLUE,
    );
    draw_text(
        &(format!("Score:{}/{}", gamestate.score[0], gamestate.score[1])),
        screen_width()
            - measure_text(
                &(format!("Score:{}/{}", gamestate.score[0], gamestate.score[1])),
                None,
                48,
                1.0,
            )
            .width
            - 10.0,
        positions[5],
        48.0,
        WHITE,
    );
}
