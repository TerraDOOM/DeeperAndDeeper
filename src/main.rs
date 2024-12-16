#![allow(dead_code, unused_variables)]

use bevy::prelude::*;

mod dating_sim;
mod game;
mod load;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Menu,
    Explore,
    DatingSim,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_plugins((
            menu::menu_plugin,
            game::game_plugin,
            dating_sim::dating_sim_plugin,
        ))
        .init_state::<GameState>()
        .run();
}

fn setup(
    mut commands: Commands,
    mut menu_state: ResMut<NextState<GameState>>,
    mut dating_state: ResMut<NextState<dating_sim::DatingState>>,
) {
    commands.spawn(Camera2d).insert(Transform::default());
    menu_state.set(GameState::DatingSim);
    dating_state.set(dating_sim::DatingState::Chilling)
}

fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn_recursive();
    }
}

mod menu {
    use super::GameState;
    use bevy::prelude::*;

    pub fn menu_plugin(app: &mut App) {
        app.add_systems(Update, change_scene.run_if(in_state(GameState::Menu)));
    }

    pub fn change_scene(
        keys: Res<ButtonInput<KeyCode>>,
        mut menu_state: ResMut<NextState<GameState>>,
    ) {
        if keys.just_pressed(KeyCode::KeyD) {
            menu_state.set(GameState::DatingSim);
            println!("going dating sim mode");
        } else if keys.just_pressed(KeyCode::KeyE) {
            menu_state.set(GameState::Explore);
            println!("going exploration mode");
        }
    }
}
