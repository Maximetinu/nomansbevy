use bevy::prelude::*;

const TIME_STEP: f32 = 1.0 / 60.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(FixedTime::new_from_secs(TIME_STEP))
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}
