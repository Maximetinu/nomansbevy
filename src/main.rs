use bevy::prelude::*;

const TIME_STEP: f32 = 1.0 / 60.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(FixedTime::new_from_secs(TIME_STEP))
        .add_systems(Startup, setup)
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let player_handle: Handle<Image> = asset_server.load("sprites/player.png");

    commands.spawn((SpriteBundle {
        texture: player_handle.clone(),
        transform: Transform::default(),
        ..default()
    },));
}
