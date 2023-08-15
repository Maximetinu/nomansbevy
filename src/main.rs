use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;
use std::ops::Range;

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
        .insert_resource(Score { current: 0 })
        .add_systems(
            Startup,
            (
                setup_artificial_gravity,
                spawn_camera,
                spawn_player,
                spawn_obstacle_spawner,
                spawn_bounds,
            ),
        )
        .add_systems(PostStartup, spawn_obstacle)
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
                game_over.run_if(on_collision::<Player, ObstacleCollider, STARTED>),
                game_over.run_if(on_collision::<Player, BoundsSensor, STOPPED>),
                score_up.run_if(on_collision::<Player, ScoreSensor, STARTED>),
                print_score.run_if(resource_changed::<Score>()),
            ),
        )
        .run();
}

fn setup_artificial_gravity(mut rapier_config: ResMut<RapierConfiguration>) {
    rapier_config.gravity = Vec2::NEG_Y * 1400.0;
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
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
}

fn spawn_obstacle_spawner(mut commands: Commands) {
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

fn spawn_bounds(mut commands: Commands) {
    commands
        .spawn(Name::new("Bounds"))
        .insert(TransformBundle::default())
        .insert(BoundsSensor {})
        .insert(Collider::cuboid(700.0, 400.0))
        .insert(Sensor {});
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct ObstacleCollider;

#[derive(Component)]
struct ScoreSensor;

#[derive(Component)]
struct BoundsSensor;

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

fn spawn_obstacle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    spawner_q: Query<(&Transform, &ObstacleSpawner)>,
) {
    let (spawner_transform, spawner) = spawner_q.single();

    let offset = rand::thread_rng().gen_range(spawner.range.clone());

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

// Workaround to generic consts only supporting primitive types
pub const STARTED: bool = true;
pub const STOPPED: bool = false;

fn on_collision<T: Component, U: Component, const COLLISION_TYPE: bool>(
    mut collision_events: EventReader<CollisionEvent>,
    first_q: Query<(Entity, &T)>,
    second_q: Query<(Entity, &U)>,
) -> bool {
    let (first_entity, _) = first_q.single();
    let mut second_entities = second_q.iter().map(|(e, _)| e);
    let mut found_collision = false;

    for collision_event in collision_events.iter() {
        match (collision_event, COLLISION_TYPE) {
            (CollisionEvent::Started(col_1, col_2, _), STARTED)
            | (CollisionEvent::Stopped(col_1, col_2, _), STOPPED) => {
                if (*col_1 == first_entity && second_entities.any(|o| o == *col_2))
                    || (*col_2 == first_entity && second_entities.any(|o| o == *col_1))
                {
                    found_collision = true;
                    break;
                }
            }
            _ => {}
        }
    }

    found_collision
}

fn score_up(mut score: ResMut<Score>) {
    score.current += 1;
}

fn print_score(score: Res<Score>) {
    println!("Current score: {}", score.current);
}

fn game_over(mut commands: Commands, windows_q: Query<(Entity, &Window)>) {
    for (window, _) in windows_q.iter() {
        commands.entity(window).despawn();
    }
}
