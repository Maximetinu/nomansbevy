use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;
use std::ops::Range;

// TODO: investigate how to implement configurable frame rate
const TIME_STEP: f32 = 1.0 / 60.0;

fn main() {
    // TODO: tweak window title and maybe size
    App::new()
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::new(),
            // TODO: tweak pixels per meter so player is 1m ?
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
            // TODO: hide debug graphics in release
            RapierDebugRenderPlugin::default(),
        ))
        .insert_resource(FixedTime::new_from_secs(TIME_STEP))
        .insert_resource(Score { current: 0 })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                bevy::window::close_on_esc,
                jump.run_if(just_pressed(KeyCode::Space)),
                tick_spawn_timer,
                spawn_obstacle.run_if(spawn_timer_just_finished),
            ),
        )
        .add_systems(
            PostUpdate,
            (
                game_over.run_if(on_collision::<Player, ObstacleCollider>),
                score_up.run_if(on_collision::<Player, ScoreSensor>),
                print_score.run_if(resource_changed::<Score>()),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    // TODO: maybe turn this into player's gravity multiplier
    rapier_config.gravity = Vec2::NEG_Y * 1400.0;

    commands.spawn(Camera2dBundle::default());

    let player_handle: Handle<Image> = asset_server.load("sprites/player.png");

    commands
        .spawn(Name::new("Player"))
        .insert(Player {})
        .insert(RigidBody::Dynamic)
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(Velocity::zero())
        .insert(Collider::ball(34.0))
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(SpriteBundle {
            texture: player_handle.clone(),
            transform: Transform::default(),
            ..default()
        });

    // TODO: spawn obstacle at startup, without having to wait 3s
    commands
        .spawn(Name::new("Obstacle Spawner"))
        .insert(ObstacleSpawner {
            timer: Timer::from_seconds(3.0, TimerMode::Repeating),
            range: -150.0..150.0,
        })
        .insert(TransformBundle::from(Transform::from_translation(
            Vec3::X * 700.0,
        )));
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct ObstacleCollider;

#[derive(Component)]
struct ScoreSensor;

#[derive(Component)]
struct ObstacleSpawner {
    pub timer: Timer,
    pub range: Range<f32>,
}

#[derive(Resource)]
struct Score {
    current: u8, // yes, u8; if player reaches 255, he beats the game
}

fn jump(mut player_q: Query<(&Player, &mut Velocity)>) {
    // TODO: replace magic numbers by constans like here
    const JUMP_VELOCITY: f32 = 400.0;
    let (_, mut player_rb) = player_q.single_mut();
    player_rb.linvel = Vec2::Y * JUMP_VELOCITY;
}

fn just_pressed(key_code: KeyCode) -> impl FnMut(Res<Input<KeyCode>>) -> bool {
    move |input: Res<Input<KeyCode>>| input.just_pressed(key_code)
}

fn tick_spawn_timer(time: Res<Time>, mut spawner_q: Query<&mut ObstacleSpawner>) {
    spawner_q.single_mut().timer.tick(time.delta());
}

fn spawn_timer_just_finished(spawner_q: Query<&ObstacleSpawner>) -> bool {
    spawner_q.single().timer.just_finished()
}

// TODO: implement bounds to despawn obstacles
// using a sensor and on_collision_exit would be nice
fn spawn_obstacle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    spawner_q: Query<(&Transform, &ObstacleSpawner)>,
) {
    let (spawner_transform, spawner) = spawner_q.single();

    let offset = rand::thread_rng().gen_range(spawner.range.clone());

    // TODO: tweak image so edges are hidden
    // TODO: investigate how to extract image size once loaded (to compute AABB, for example)
    let obstacle_handle: Handle<Image> = asset_server.load("sprites/obstacle.png");

    commands
        .spawn(Name::new("Obstacle"))
        .insert(RigidBody::KinematicVelocityBased)
        .insert(Velocity::linear(Vec2::NEG_X * 150.0))
        .insert(SpatialBundle::from_transform(Transform::from_translation(
            spawner_transform.translation + Vec3::Y * offset,
        )))
        .with_children(|children| {
            children
                .spawn(Name::new("Obstacle Up"))
                .insert(ObstacleCollider {})
                .insert(Collider::cuboid(32.0, 128.0))
                .insert(SpriteBundle {
                    texture: obstacle_handle.clone(),
                    transform: Transform::from_translation(Vec3::Y * 250.0),
                    ..default()
                });
            children
                .spawn(Name::new("Score sensor"))
                .insert(ScoreSensor {})
                .insert(TransformBundle::default())
                .insert(Collider::cuboid(10.0, 122.0))
                .insert(Sensor);
            children
                .spawn(Name::new("Obstacle Down"))
                .insert(ObstacleCollider {})
                .insert(Collider::cuboid(32.0, 128.0))
                .insert(SpriteBundle {
                    texture: obstacle_handle.clone(),
                    transform: Transform::from_translation(Vec3::NEG_Y * 250.0),
                    ..default()
                });
        });
}

fn on_collision<T: Component, U: Component>(
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

fn score_up(mut score: ResMut<Score>) {
    score.current += 1;
}

fn print_score(score: Res<Score>) {
    // TODO: UI
    println!("Current score: {}", score.current);
}

fn game_over(mut commands: Commands, windows_q: Query<(Entity, &Window)>) {
    // TODO: implement reset level
    for (window, _) in windows_q.iter() {
        commands.entity(window).despawn();
    }
}
