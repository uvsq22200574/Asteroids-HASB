use crate::general::Gamestate;
use crate::helpers::Entity;
use rdev::{listen, Button, Event, EventType, Key};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs::{read_to_string, write},
    sync::{Arc, Mutex},
    thread::spawn,
};

/// Scroll state used internally
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollState {
    Idle,
    Up,
    Down,
}

impl Default for ScrollState {
    fn default() -> Self {
        ScrollState::Idle
    }
}

/// Serializable wrapper for keyboard keys or mouse action
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyInput {
    Key(String),
    Mouse(String),
    Scroll(String),
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
    pub bindings: HashMap<Action, Vec<KeyCombo>>,

    #[serde(skip)]
    pressed_keys: Arc<Mutex<Vec<String>>>,
    #[serde(skip)]
    just_pressed: Arc<Mutex<Vec<String>>>,
    #[serde(skip)]
    just_released: Arc<Mutex<Vec<String>>>,
    #[serde(skip)]
    scroll_state: Arc<Mutex<ScrollState>>,
    #[serde(skip)]
    scroll_sensitivity: f64,
    #[serde(skip)]
    scroll_accumulator_up: Arc<Mutex<f64>>,
    #[serde(skip)]
    scroll_accumulator_down: Arc<Mutex<f64>>,
}

impl KeyBindings {
    /// Create empty keybindings
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            pressed_keys: Arc::new(Mutex::new(Vec::new())),
            just_pressed: Arc::new(Mutex::new(Vec::new())),
            just_released: Arc::new(Mutex::new(Vec::new())),
            scroll_state: Arc::new(Mutex::new(ScrollState::Idle)),
            scroll_sensitivity: 1.0,
            scroll_accumulator_up: Arc::new(Mutex::new(0.0)),
            scroll_accumulator_down: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Bind a key combo to an action
    pub fn bind(&mut self, action: Action, combo: KeyCombo) {
        self.bindings.entry(action).or_default().push(combo);
    }

    /// Bind a single keyboard key
    pub fn bind_key(&mut self, action: Action, key: Key) {
        self.bind(
            action,
            KeyCombo {
                input: KeyInput::from_key(key),
                modifiers: Vec::new(),
            },
        );
    }

    /// Bind a single mouse button
    pub fn bind_mouse(&mut self, action: Action, button: Button) {
        self.bind(
            action,
            KeyCombo {
                input: KeyInput::from_button(button),
                modifiers: Vec::new(),
            },
        );
    }

    /// Call this at the end of your main loop to clear transient states
    pub fn clear_events(&self) {
        self.just_pressed.lock().unwrap().clear();
        self.just_released.lock().unwrap().clear();
        *self.scroll_state.lock().unwrap() = ScrollState::Idle;
    }

    /// Start listening to global keyboard and mouse events
    pub fn start_listening(&self) {
        let pressed_clone = Arc::clone(&self.pressed_keys);
        let just_pressed_clone = Arc::clone(&self.just_pressed);
        let just_released_clone = Arc::clone(&self.just_released);
        let scroll_state_clone = Arc::clone(&self.scroll_state);
        let sensitivity = self.scroll_sensitivity;
        let scroll_accumulator_up = Arc::clone(&self.scroll_accumulator_up);
        let scroll_accumulator_down = Arc::clone(&self.scroll_accumulator_down);

        spawn(move || {
            listen(move |event: Event| match event.event_type {
                EventType::KeyPress(k) => {
                    let k_str = format!("{:?}", k);
                    let mut pressed = pressed_clone.lock().unwrap();
                    let mut just_pressed = just_pressed_clone.lock().unwrap();
                    if !pressed.contains(&k_str) {
                        pressed.push(k_str.clone());
                        just_pressed.push(k_str);
                    }
                }
                EventType::KeyRelease(k) => {
                    let k_str = format!("{:?}", k);
                    let mut pressed = pressed_clone.lock().unwrap();
                    let mut just_released = just_released_clone.lock().unwrap();
                    pressed.retain(|x| x != &k_str);
                    just_released.push(k_str);
                }
                EventType::ButtonPress(b) => {
                    let k_str = format!("{:?}", b);
                    let mut pressed = pressed_clone.lock().unwrap();
                    let mut just_pressed = just_pressed_clone.lock().unwrap();
                    if !pressed.contains(&k_str) {
                        pressed.push(k_str.clone());
                        just_pressed.push(k_str);
                    }
                }
                EventType::ButtonRelease(b) => {
                    let k_str = format!("{:?}", b);
                    let mut pressed = pressed_clone.lock().unwrap();
                    let mut just_released = just_released_clone.lock().unwrap();
                    pressed.retain(|x| x != &k_str);
                    just_released.push(k_str);
                }
                EventType::Wheel {
                    delta_x: _,
                    delta_y,
                } => {
                    let delta_y_f = delta_y as f64;
                    let mut scroll_state = scroll_state_clone.lock().unwrap();
                    let mut just_pressed = just_pressed_clone.lock().unwrap();

                    if delta_y_f > 0.0 {
                        *scroll_state = ScrollState::Up;
                        let mut acc = scroll_accumulator_up.lock().unwrap();
                        *acc += delta_y_f * sensitivity;
                        let change = *acc as i32;
                        if change != 0 {
                            just_pressed.push(format!("ScrollUp:{change}"));
                            *acc -= change as f64; // keep remainder
                        }
                    } else if delta_y_f < 0.0 {
                        *scroll_state = ScrollState::Down;
                        let mut acc = scroll_accumulator_down.lock().unwrap();
                        *acc += delta_y_f.abs() * sensitivity;
                        let change = *acc as i32;
                        if change != 0 {
                            just_pressed.push(format!("ScrollDown:{change}"));
                            *acc -= change as f64;
                        }
                    } else {
                        *scroll_state = ScrollState::Idle;
                    }
                }

                _ => {}
            })
            .unwrap();
        });
    }

    /// Save bindings to a JSON file in a predictable (sorted) manner
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Convert into a BTreeMap for sorted keys
        let sorted: BTreeMap<_, _> = self.bindings.iter().collect();
        let json = serde_json::to_string_pretty(&sorted)?;
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

    // === Setter | Getters ===
    pub fn set_scroll_sensitivity(&mut self, sensitivity: f64) {
        self.scroll_sensitivity = sensitivity.max(0.1);
    }

    /// Get a clone of the currently pressed keys
    pub fn get_held_keys(&self) -> Vec<String> {
        let mut pressed = self.pressed_keys.lock().unwrap().clone();
        let scroll_state = self.scroll_state.lock().unwrap();
        let scroll_str = match *scroll_state {
            ScrollState::Idle => "Scroll_Idle",
            ScrollState::Up => "Scroll_Up",
            ScrollState::Down => "Scroll_Down",
        };
        pressed.push(scroll_str.to_string());
        pressed
    }

    pub fn is_action_held(&self, action: Action) -> bool {
        let pressed = self.pressed_keys.lock().unwrap();
        self.is_combo_active(&pressed, action)
    }

    pub fn is_action_pressed(&self, action: Action) -> bool {
        let just_pressed = self.just_pressed.lock().unwrap();
        self.is_combo_active(&just_pressed, action)
    }

    pub fn is_action_released(&self, action: Action) -> bool {
        let just_released = self.just_released.lock().unwrap();
        self.is_combo_active(&just_released, action)
    }

    /// Helper to check if an action's key combinations are active.
    ///
    /// A combination is considered **active** if:
    /// - All of its modifiers are currently in the pressed set.
    /// - Its main key / mouse button / scroll event is also in the pressed set.
    ///
    /// # Example
    /// Binding `Ctrl + S` to a `SaveAction`:
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use crate::key_bindings::{KeyBindings, KeyCombo, KeyInput};
    ///
    /// // Create a keybinding instance
    /// let mut kb = KeyBindings::new();
    ///
    /// // Bind Ctrl+S to "SaveAction"
    /// kb.bind(
    ///     "SaveAction",
    ///     KeyCombo {
    ///         input: KeyInput::Key("KeyS".into()),
    ///         modifiers: vec!["ControlLeft".into()],
    ///     },
    /// );
    ///
    /// // Simulate currently pressed keys
    /// let pressed = vec!["ControlLeft".to_string(), "KeyS".to_string()];
    ///
    /// // Call the helper directly
    /// let active = kb.is_combo_active(&pressed, "SaveAction");
    /// assert!(active); // true, because Ctrl+S is pressed
    /// ```
    ///
    /// In normal usage you don’t call this function directly;
    /// instead you use higher-level helpers like:
    ///
    /// ```
    /// if keybindings.is_action_pressed("SaveAction") {
    ///     println!("Save triggered!");
    /// }
    /// ```
    ///
    fn is_combo_active(&self, set: &Vec<String>, action: Action) -> bool {
        if let Some(combos) = self.bindings.get(&action) {
            for combo in combos {
                let all_modifiers_pressed = combo.modifiers.iter().all(|m| set.contains(m));
                let main_pressed = match &combo.input {
                    KeyInput::Key(k) | KeyInput::Mouse(k) | KeyInput::Scroll(k) => set.contains(k),
                };
                if all_modifiers_pressed && main_pressed {
                    return true;
                }
            }
        }
        false
    }
}

pub fn default_keybindings() -> KeyBindings {
    let mut kb = KeyBindings::new();

    kb.bind_key(Action::SpeedUp, Key::KeyW);
    kb.bind_key(Action::SpeedDown, Key::KeyS);
    kb.bind_key(Action::MoveLeft, Key::KeyA);
    kb.bind_key(Action::MoveRight, Key::KeyD);
    kb.bind_mouse(Action::Stop, Button::Middle);

    kb.bind_mouse(Action::Fire, Button::Left);
    kb.bind_mouse(Action::FireHoming, Button::Right);
    kb.bind_key(Action::Fire, Key::KeyQ);
    kb.bind_key(Action::FireHoming, Key::KeyE);

    kb.bind_key(Action::ToggleDebug, Key::F3);
    kb.bind_key(Action::Confirm, Key::Return);
    kb.bind_key(Action::Escape, Key::Escape);

    kb.bind_key(Action::Pause, Key::Space);
    kb.bind_key(Action::Accelerate, Key::Tab);
    kb.bind_key(Action::SlowDown, Key::ShiftLeft);

    // Scroll
    kb.bind(
        Action::ScrollUp,
        KeyCombo {
            input: KeyInput::Scroll("ScrollUp".into()),
            modifiers: Vec::new(),
        },
    );
    kb.bind(
        Action::ScrollDown,
        KeyCombo {
            input: KeyInput::Scroll("ScrollDown".into()),
            modifiers: Vec::new(),
        },
    );

    kb
}

/// Game actions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
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

    Pause,
    Accelerate,
    SlowDown,

    ScrollUp,
    ScrollDown,
}

pub fn handle_input(gamestate: &mut Gamestate, keybindings: &KeyBindings) {
    let turn_rate = gamestate.spaceship.get_turn_rate();

    // Toggle debug
    if keybindings.is_action_pressed(Action::ToggleDebug) {
        gamestate.debug = !gamestate.debug;
    }

    // Start menu handling
    if gamestate.get_last_menu_item() == "Start" {
        if keybindings.is_action_pressed(Action::Confirm) {
            gamestate.reset();
        }
        if keybindings.is_action_pressed(Action::Escape) {
            keybindings.clear_events();
            gamestate.exit = true;
        }
    }

    // Pause menu
    if keybindings.is_action_pressed(Action::Escape) {
        if gamestate.menu.is_empty() {
            gamestate.menu.push(String::from("Main"));
        } else {
            gamestate.menu.pop();
        }
    }

    // Thrust forward/backward
    if keybindings.is_action_held(Action::SpeedUp) && gamestate.simulation_speed > 0.0 {
        gamestate
            .spaceship
            .move_spaceship(gamestate.delta_time, true);
    }
    if keybindings.is_action_held(Action::SpeedDown) && gamestate.simulation_speed > 0.0 {
        gamestate
            .spaceship
            .move_spaceship(gamestate.delta_time, false);
    }

    // Rotation
    if keybindings.is_action_held(Action::MoveLeft) && gamestate.simulation_speed > 0.0 {
        gamestate
            .spaceship
            .add_rotation(-turn_rate * gamestate.delta_time as f32);
    }
    if keybindings.is_action_held(Action::MoveRight) && gamestate.simulation_speed > 0.0 {
        gamestate
            .spaceship
            .add_rotation(turn_rate * gamestate.delta_time as f32);
    }

    // Stop
    if keybindings.is_action_pressed(Action::Stop) {
        gamestate.spaceship.stop();
    }

    // Fire missiles
    if gamestate.spaceship.get_life()
        && gamestate.spaceship.get_firing_cooldown() <= 0.0
        && keybindings.is_action_held(Action::Fire)
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
        && keybindings.is_action_held(Action::FireHoming)
        && gamestate.simulation_speed > 0.0
        && (gamestate.debug || gamestate.menu.is_empty())
    {
        gamestate.summon_missile(true);
        gamestate.spaceship.set_homming_cooldown(0.8);
    }

    // Scroll handling (capacity changes)
    // One-shot actions
    if keybindings.is_action_pressed(Action::ScrollUp) && gamestate.simulation_speed > 0.0 {
        gamestate.spaceship.modify_capacity(1);
    }
    if keybindings.is_action_pressed(Action::ScrollDown) && gamestate.simulation_speed > 0.0 {
        gamestate.spaceship.modify_capacity(-1);
    }

    // Accumulated scroll changes
    for key in keybindings.just_pressed.lock().unwrap().iter() {
        if key.starts_with("ScrollUp:") {
            let value: i8 = key["ScrollUp:".len()..].parse().unwrap();
            gamestate.spaceship.modify_capacity(value);
        } else if key.starts_with("ScrollDown:") {
            let value: i8 = key["ScrollDown:".len()..].parse().unwrap();
            gamestate.spaceship.modify_capacity(-value);
        }
    }

    // Time manipulation
    if keybindings.is_action_held(Action::Pause) {
        gamestate.simulation_speed = 0.0;
    }
    if keybindings.is_action_held(Action::Accelerate) {
        gamestate.simulation_speed = 5.0;
    }
    if keybindings.is_action_held(Action::SlowDown) {
        gamestate.simulation_speed = 0.075;
    }

    keybindings.clear_events();
}
