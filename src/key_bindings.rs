use crate::general::Gamestate;
use crate::helpers::Entity;
use rdev::{listen, Button, Event, EventType, Key};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{read_to_string, write},
    sync::{Arc, Mutex},
    thread::spawn,
};

/// Serializable wrapper for keyboard keys or mouse buttons
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyInput {
    Key(String),
    Mouse(String),
}

impl KeyInput {
    pub fn from_key(k: Key) -> Self {
        KeyInput::Key(format!("{:?}", k))
    }

    pub fn from_button(b: Button) -> Self {
        KeyInput::Mouse(format!("{:?}", b))
    }
}

/// Represents a key or key+modifier combination
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    pub input: KeyInput,
    pub modifiers: Vec<String>, // modifiers like Ctrl, Shift, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub bindings: HashMap<String, Vec<KeyCombo>>,

    #[serde(skip)]
    pressed_keys: Arc<Mutex<Vec<String>>>,
    #[serde(skip)]
    just_pressed: Arc<Mutex<Vec<String>>>,
    #[serde(skip)]
    just_released: Arc<Mutex<Vec<String>>>,
}

impl KeyBindings {
    /// Create empty keybindings
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            pressed_keys: Arc::new(Mutex::new(Vec::new())),
            just_pressed: Arc::new(Mutex::new(Vec::new())),
            just_released: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Bind a key combo to an action
    pub fn bind(&mut self, action: &str, combo: KeyCombo) {
        self.bindings
            .entry(action.to_string())
            .or_default()
            .push(combo);
    }

    /// Bind a single keyboard key
    pub fn bind_key(&mut self, action: &str, key: Key) {
        self.bind(
            action,
            KeyCombo {
                input: KeyInput::from_key(key),
                modifiers: Vec::new(),
            },
        );
    }

    /// Bind a single mouse button
    pub fn bind_mouse(&mut self, action: &str, button: Button) {
        self.bind(
            action,
            KeyCombo {
                input: KeyInput::from_button(button),
                modifiers: Vec::new(),
            },
        );
    }

    /// Check if action is currently held
    pub fn is_action_held(&self, action: &str) -> bool {
        let pressed = self.pressed_keys.lock().unwrap();
        self.is_combo_active(&pressed, action)
    }

    /// Check if action was just pressed (this event loop tick)
    pub fn is_action_pressed(&self, action: &str) -> bool {
        let just_pressed = self.just_pressed.lock().unwrap();
        self.is_combo_active(&just_pressed, action)
    }

    /// Check if action was just released
    pub fn is_action_released(&self, action: &str) -> bool {
        let just_released = self.just_released.lock().unwrap();
        self.is_combo_active(&just_released, action)
    }

    /// Helper to check combos against a set of active keys
    fn is_combo_active(&self, set: &Vec<String>, action: &str) -> bool {
        if let Some(combos) = self.bindings.get(action) {
            for combo in combos {
                if combo.modifiers.iter().all(|m| set.contains(m)) {
                    match &combo.input {
                        KeyInput::Key(k) | KeyInput::Mouse(k) => {
                            if set.contains(k) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Call this at the end of your main loop to clear transient states
    pub fn clear_events(&self) {
        self.just_pressed.lock().unwrap().clear();
        self.just_released.lock().unwrap().clear();
    }

    /// Start listening to global keyboard and mouse events
    pub fn start_listening(&self) {
        let pressed_clone = Arc::clone(&self.pressed_keys);
        let just_pressed_clone = Arc::clone(&self.just_pressed);
        let just_released_clone = Arc::clone(&self.just_released);

        // Create a thread
        spawn(move || {
            listen(move |event: Event| {
                let key_str = match event.event_type {
                    EventType::KeyPress(k) => Some(format!("{:?}", k)),
                    EventType::KeyRelease(k) => Some(format!("{:?}", k)),
                    EventType::ButtonPress(b) => Some(format!("{:?}", b)),
                    EventType::ButtonRelease(b) => Some(format!("{:?}", b)),
                    _ => None,
                };

                if let Some(k) = key_str {
                    let mut pressed = pressed_clone.lock().unwrap();
                    match event.event_type {
                        EventType::KeyPress(_) | EventType::ButtonPress(_) => {
                            if !pressed.contains(&k) {
                                pressed.push(k.clone());
                                just_pressed_clone.lock().unwrap().push(k);
                            }
                        }
                        EventType::KeyRelease(_) | EventType::ButtonRelease(_) => {
                            pressed.retain(|x| x != &k);
                            just_released_clone.lock().unwrap().push(k);
                        }
                        _ => {}
                    }
                }
            })
            .unwrap();
        });
    }

    /// Get a clone of the currently pressed keys
    pub fn get_held_keys(&self) -> Vec<String> {
        let pressed = self.pressed_keys.lock().unwrap();
        pressed.clone()
    }

    /// Save bindings to a JSON file
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        write(path, json)?;
        Ok(())
    }

    /// Load bindings from a JSON file
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = read_to_string(path)?;
        let mut bindings: KeyBindings = serde_json::from_str(&data)?;
        bindings.pressed_keys = Arc::new(Mutex::new(Vec::new()));
        Ok(bindings)
    }
}

/// Default keybindings
pub fn default_keybindings() -> KeyBindings {
    let mut kb = KeyBindings::new();

    kb.bind_key("SpeedUp", Key::KeyW);
    kb.bind_key("SpeedDown", Key::KeyS);
    kb.bind_key("MoveLeft", Key::KeyA);
    kb.bind_key("MoveRight", Key::KeyD);

    kb.bind_mouse("Fire", Button::Left);
    kb.bind_mouse("FireHoming", Button::Right);
    kb.bind_key("Fire", Key::KeyQ);
    kb.bind_key("FireHoming", Key::KeyE);

    kb.bind_key("ToggleDebug", Key::F3);
    kb.bind_key("Confirm", Key::Return);
    kb.bind_key("Escape", Key::Escape);
    kb.bind_key("Upgrade", Key::KeyI);

    // Time Manipulation
    kb.bind_key("Pause", Key::Space);
    kb.bind_key("Accelerate", Key::Tab);
    kb.bind_key("SlowDown", Key::ShiftLeft);

    kb
}

/// Game actions
#[derive(serde::Deserialize, Debug)]
pub enum Action {
    SpeedUp,
    SpeedDown,
    MoveLeft,
    MoveRight,
    Stop,
    Fire,
    FireHoming,
    ToggleDebug,
    Escape,
    Confirm,
    Upgrade,
    Pause,
    Accelerate,
    SlowDown,
}

pub fn handle_input(gamestate: &mut Gamestate, keybindings: &KeyBindings) {
    let turn_rate = gamestate.spaceship.get_turn_rate();

    // Helper to convert enum to string key
    let action_str = |action: &Action| format!("{:?}", action);

    if keybindings.is_action_pressed(&action_str(&Action::ToggleDebug)) {
        gamestate.debug = !gamestate.debug;
    }

    // Handles the start menu
    if gamestate.get_last_menu_item() == "Start" {
        if keybindings.is_action_pressed(&action_str(&Action::Confirm)) {
            gamestate.reset();
        }
        if keybindings.is_action_pressed(&action_str(&Action::Escape)) {
            keybindings.clear_events();
            gamestate.exit = true;
        }
    }

    // Pause menu
    if keybindings.is_action_pressed(&action_str(&Action::Escape)) {
        if gamestate.menu.is_empty() {
            gamestate.menu.push(String::from("Main"));
        } else {
            gamestate.menu.pop();
        }
    }

    // Thrust forward/backward
    if keybindings.is_action_held(&action_str(&Action::SpeedUp)) && gamestate.simulation_speed > 0.0
    {
        gamestate
            .spaceship
            .move_spaceship(gamestate.delta_time, true);
    }
    if keybindings.is_action_held(&action_str(&Action::SpeedDown))
        && gamestate.simulation_speed > 0.0
    {
        gamestate
            .spaceship
            .move_spaceship(gamestate.delta_time, false);
    }

    // Rotate
    if keybindings.is_action_held(&action_str(&Action::MoveLeft))
        && gamestate.simulation_speed > 0.0
    {
        gamestate
            .spaceship
            .add_rotation(-turn_rate * gamestate.delta_time as f32);
    }
    if keybindings.is_action_held(&action_str(&Action::MoveRight))
        && gamestate.simulation_speed > 0.0
    {
        gamestate
            .spaceship
            .add_rotation(turn_rate * gamestate.delta_time as f32);
    }

    // Fire normal missile
    if gamestate.spaceship.get_life()
        && gamestate.spaceship.get_firing_cooldown() <= 0.0
        && keybindings.is_action_held(&action_str(&Action::Fire))
        && gamestate.simulation_speed > 0.0
        && (gamestate.debug || gamestate.menu.is_empty())
    {
        gamestate.summon_missile(false);
        gamestate.spaceship.set_firing_cooldown(0.15);
    }

    // Fire homing missiles
    if gamestate.spaceship.get_life()
        && gamestate.spaceship.get_missile_capacity() > 0
        && gamestate.spaceship.get_homming_cooldown() <= 0.0
        && keybindings.is_action_held(&action_str(&Action::FireHoming))
        && gamestate.simulation_speed > 0.0
        && (gamestate.debug || gamestate.menu.is_empty())
    {
        gamestate.summon_missile(true);
        gamestate.spaceship.set_homming_cooldown(0.8);
    }

    if keybindings.is_action_pressed("Upgrade") && gamestate.simulation_speed > 0.0 {
        gamestate.spaceship.modify_capacity(1);
    }

    // Time manipulation
    if keybindings.is_action_held(&action_str(&Action::Pause)) {
        gamestate.simulation_speed = 0.0;
    }

    if keybindings.is_action_held(&action_str(&Action::Accelerate)) {
        gamestate.simulation_speed = 5.0;
    }

    if keybindings.is_action_held(&action_str(&Action::SlowDown)) {
        gamestate.simulation_speed = 0.075;
    }

    keybindings.clear_events();
}
