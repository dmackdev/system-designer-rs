use bevy::prelude::States;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Loading,
    MainMenu,
    LevelSelect,
    Edit,
    Validate,
    Simulate,
    SimulateFinish,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameMode {
    #[default]
    Levels,
    Sandbox,
}
