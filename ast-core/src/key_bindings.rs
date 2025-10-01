use crate::gamestate::Gamestate;
use ast_lib::CosmicEntity;
use std::{
    collections::{BTreeMap, HashMap},
    fs::{read_to_string, write},
    sync::{Arc, Mutex},
    thread::spawn,
};
use rdev::{listen, Button, Event, EventType, Key};
use serde::{Deserialize, Serialize};

// === DEFINITIONS ===

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

    pub fn as_string(&self) -> &str {
        match self {
            KeyInput::Key(k) | KeyInput::Mouse(k) | KeyInput::Scroll(k) => k,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modifier {
    Control,
    Shift,
    Alt,
    Meta,
}

impl std::fmt::Display for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Modifier::Control => "Control",
            Modifier::Shift => "Shift",
            Modifier::Alt => "Alt",
            Modifier::Meta => "Meta",
        };
        write!(f, "{}", s)
    }
}

/// Represents a key or key+modifier combination
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    pub input: KeyInput,
    pub modifiers: Vec<Modifier>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    pub bindings: HashMap<Action, Vec<KeyCombo>>,

    #[serde(skip)]
    listener_handle: Option<std::thread::JoinHandle<()>>,
    #[serde(skip)]
    input_state: Arc<Mutex<InputState>>,
    #[serde(skip)]
    scroll_state: Arc<Mutex<ScrollState>>,
    #[serde(skip)]
    scroll_accumulator: Arc<Mutex<f64>>,
    #[serde(skip)]
    scroll_sensitivity: f64,
}

#[derive(Debug, Clone, Default)]
struct InputState {
    pressed: Vec<String>,
    just_pressed: Vec<String>,
    just_released: Vec<String>,
}

#[allow(unused)]
impl KeyBindings {
    /// Create empty keybindings
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            listener_handle: None,
            input_state: Arc::new(Mutex::new(InputState::default())),
            scroll_state: Arc::new(Mutex::new(ScrollState::Idle)),
            scroll_accumulator: Arc::new(Mutex::new(0.0)),
            scroll_sensitivity: 1.0,
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
        let mut input = self.input_state.lock().unwrap();
        input.just_pressed.clear();
        input.just_released.clear();
        *self.scroll_state.lock().unwrap() = ScrollState::Idle;
    }

    /// Start listening to global keyboard and mouse events
    pub fn start_listening(&self) {
        let input_state_clone = Arc::clone(&self.input_state);
        let scroll_state_clone = Arc::clone(&self.scroll_state);
        let scroll_accumulator_clone = Arc::clone(&self.scroll_accumulator); // <-- clone the Arc
        let sensitivity = self.scroll_sensitivity;

        // Spawn a separate thread for the global listener
        spawn(move || {
            listen(move |event: Event| {
                let mut input = input_state_clone.lock().unwrap();

                match event.event_type {
                    EventType::KeyPress(k) => {
                        let k_str = format!("{:?}", k);
                        if !input.pressed.contains(&k_str) {
                            input.pressed.push(k_str.clone());
                            input.just_pressed.push(k_str);
                        }
                    }
                    EventType::KeyRelease(k) => {
                        let k_str = format!("{:?}", k);
                        input.pressed.retain(|x| x != &k_str);
                        input.just_released.push(k_str);
                    }
                    EventType::ButtonPress(b) => {
                        let b_str = format!("{:?}", b);
                        if !input.pressed.contains(&b_str) {
                            input.pressed.push(b_str.clone());
                            input.just_pressed.push(b_str);
                        }
                    }
                    EventType::ButtonRelease(b) => {
                        let b_str = format!("{:?}", b);
                        input.pressed.retain(|x| x != &b_str);
                        input.just_released.push(b_str);
                    }
                    EventType::Wheel {
                        delta_x: _,
                        delta_y,
                    } => {
                        let mut scroll_state = scroll_state_clone.lock().unwrap();
                        let mut acc = scroll_accumulator_clone.lock().unwrap();
                        let delta_f = delta_y as f64 * sensitivity;

                        *acc += delta_f; // accumulate positive and negative values

                        if *acc > 0.0 {
                            *scroll_state = ScrollState::Up;
                            let change = *acc as i32;
                            if change != 0 {
                                input.just_pressed.push(format!("ScrollUp:{change}"));
                                *acc -= change as f64; // keep remainder
                            }
                        } else if *acc < 0.0 {
                            *scroll_state = ScrollState::Down;
                            let change = (-*acc) as i32;
                            if change != 0 {
                                input.just_pressed.push(format!("ScrollDown:{change}"));
                                *acc += change as f64; // keep remainder
                            }
                        } else {
                            *scroll_state = ScrollState::Idle;
                        }
                    }

                    _ => {}
                }
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
        let mut bindings: KeyBindings = serde_json::from_str(&read_to_string(path)?)?;
        bindings.input_state = Arc::new(Mutex::new(InputState::default()));
        bindings.scroll_state = Arc::new(Mutex::new(ScrollState::Idle));
        bindings.scroll_accumulator = Arc::new(Mutex::new(0.0));
        bindings.listener_handle = None;
        Ok(bindings)
    }

    // === Setter | Getters ===
    pub fn set_scroll_sensitivity(&mut self, sensitivity: f64) {
        self.scroll_sensitivity = sensitivity.max(0.1);
    }

    /// Get a clone of the currently pressed keys
    pub fn get_held_keys(&self) -> Vec<String> {
        let input = self.input_state.lock().unwrap();
        let mut pressed = input.pressed.clone(); // clone the current pressed keys
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
        let input = self.input_state.lock().unwrap();
        self.is_combo_active(&input.pressed, action)
    }

    pub fn is_action_pressed(&self, action: Action) -> bool {
        let input = self.input_state.lock().unwrap();
        self.is_combo_active(&input.just_pressed, action)
    }

    pub fn is_action_released(&self, action: Action) -> bool {
        let input = self.input_state.lock().unwrap();
        self.is_combo_active(&input.just_released, action)
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
    fn is_combo_active(&self, set: &[String], action: Action) -> bool {
        if let Some(combos) = self.bindings.get(&action) {
            for combo in combos {
                let all_modifiers_pressed =
                    combo.modifiers.iter().all(|m| set.contains(&m.to_string()));
                let main_pressed = set.contains(&String::from(combo.input.as_string()));
                if all_modifiers_pressed && main_pressed {
                    return true;
                }
            }
        }
        false
    }
}

// === END DEFINITION ===

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

pub fn handle_input(gamestate: &mut Gamestate, keybindings: &KeyBindings) {
    let turn_rate = gamestate.spaceship.get_turn_rate();
    let input_snapshot = keybindings.input_state.lock().unwrap().clone();

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
    for key in &input_snapshot.just_pressed {
        if let Some(rest) = key.strip_prefix("ScrollUp:") {
            if let Ok(value) = rest.parse::<i8>() {
                gamestate.spaceship.modify_capacity(value);
            }
        } else if let Some(rest) = key.strip_prefix("ScrollDown:") {
            if let Ok(value) = rest.parse::<i8>() {
                gamestate.spaceship.modify_capacity(-value);
            }
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
