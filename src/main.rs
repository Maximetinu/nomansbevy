use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;

const TIME_STEP: f32 = 1.0 / 60.0;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::new(),
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
            RapierDebugRenderPlugin::default(),
        ))
        .insert_resource(FixedTime::new_from_secs(TIME_STEP))
        .add_systems(Startup, setup)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, jump)
        .add_systems(PostUpdate, print_collisions)
        .add_systems(
            PostUpdate,
            handle_player_obstacle_collision.run_if(on_collision_enter::<Player, Obstacle>),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    rapier_config.gravity = Vec2::NEG_Y * 1400.0;

    commands.spawn(Camera2dBundle::default());

    let player_handle: Handle<Image> = asset_server.load("sprites/player.png");

    // ball
    commands
        .spawn(Name::new("Player"))
        .insert(Player {})
        .insert(RigidBody::Dynamic)
        .insert(Velocity::zero())
        .insert(Collider::ball(34.0))
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(SpriteBundle {
            texture: player_handle.clone(),
            transform: Transform::default(),
            ..default()
        });

    // ground
    commands
        .spawn(Name::new("Ground"))
        .insert(Obstacle {})
        .insert(Collider::cuboid(500.0, 50.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -200.0, 0.0)));

    // Note: .insert(Sensor) makes it a Trigger
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Obstacle;

fn jump(input: Res<Input<KeyCode>>, mut player_q: Query<(&Player, &mut Velocity)>) {
    const JUMP_VELOCITY: f32 = 400.0;
    if !input.just_pressed(KeyCode::Space) {
        return;
    }
    let (_, mut player_rb) = player_q.single_mut();
    player_rb.linvel = Vec2::Y * JUMP_VELOCITY;
}

fn print_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    mut contact_force_events: EventReader<ContactForceEvent>,
) {
    for collision_event in collision_events.iter() {
        println!(">> Received collision event: {collision_event:?}");
    }

    for contact_force_event in contact_force_events.iter() {
        println!(">> Received contact force event: {contact_force_event:?}");
    }
}

fn on_collision_enter<T: Component, U: Component>(
    mut collision_events: EventReader<CollisionEvent>,
    first_q: Query<(Entity, &T)>,
    second_q: Query<(Entity, &U)>,
) -> bool {
    let (first_entity, _) = first_q.single();
    let mut second_entities = second_q.iter().map(|(e, _)| e);
    let mut found_collision = false;

    for collision_event in collision_events.iter() {
        match collision_event {
            CollisionEvent::Started(col_1, col_2, _) => {
                if (*col_1 == first_entity && second_entities.any(|o| o == *col_2))
                    || (*col_2 == first_entity && second_entities.any(|o| o == *col_1))
                {
                    found_collision = true;
                    break;
                }
            }
            CollisionEvent::Stopped(_, _, _) => {}
        }
    }

    found_collision
}

fn handle_player_obstacle_collision() {
    println!(">> Player collided with an obstacle just now");
}
