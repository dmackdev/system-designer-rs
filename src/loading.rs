use bevy::{asset::LoadState, prelude::*};

use crate::{game_state::AppState, Handles};

pub struct LoadingPlugin;

pub const NUM_LEVELS: u32 = 1;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Handles>();
        app.add_system(loading_setup.in_schedule(OnEnter(AppState::Loading)));
        app.add_system(loading_update.in_set(OnUpdate(AppState::Loading)));
    }
}

fn loading_setup(mut handles: ResMut<Handles>, asset_server: Res<AssetServer>) {
    for i in 1..=NUM_LEVELS {
        handles
            .levels
            .push(asset_server.load(format!("levels/{i}.level.ron").as_str()));
    }
}

fn loading_update(
    handles: Res<Handles>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !matches!(
        asset_server.get_group_load_state(handles.levels.iter().cloned().map(|h| h.id())),
        LoadState::Loaded
    ) {
        return;
    }

    next_state.set(AppState::MainMenu);
}
