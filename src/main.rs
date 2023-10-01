use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;
use std::ops::Range;

/*
Fully aware that many of the patterns below are not Bevy/ECS idiomatic
e.g. having an "obstacle spawner" entity instead of a system/resource
or having a sensor bounds instead of just screen size...
Damn, even using physics for such detection is overkilling
So, big reminder that this is just experimentation !!!
*/

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
        .insert_resource(Score(0))
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
                game_over.run_if(on_collision::<Player, ObstaclePart, Started>),
                game_over.run_if(on_collision::<Player, Bounds, Stopped>),
                score_up.run_if(on_collision::<Player, ObstacleScore, Started>),
                print_score.run_if(resource_changed::<Score>()),
                get_collisions::<ObstacleParent, Bounds, Stopped>
                    .pipe(map_entity_pairs_lhs)
                    .pipe(despawn),
            ),
        )
        .run();
}

fn map_entity_pairs_lhs(In(entity_pairs): In<Vec<(Entity, Entity)>>) -> Vec<Entity> {
    entity_pairs.iter().map(|(lhs, _)| lhs).cloned().collect()
}

fn despawn(In(entities): In<Vec<Entity>>, mut commands: Commands) {
    for e in entities {
        commands.entity(e).despawn_recursive();
    }
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
        .insert(Player)
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
        .insert(Bounds)
        .insert(Collider::cuboid(700.0, 400.0))
        .insert(Sensor);
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct ObstaclePart;

#[derive(Component)]
struct ObstacleParent;

#[derive(Component)]
struct ObstacleScore;

#[derive(Component)]
struct Bounds;

#[derive(Component)]
struct ObstacleSpawner {
    pub timer: Timer,
    pub range: Range<f32>,
}

#[derive(Resource, Deref, DerefMut)]
struct Score(u8); // yes, u8; if player reaches 255, he beats the game

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

    let offset = thread_rng().gen_range(spawner.range.clone());

    let obstacle_handle: Handle<Image> = asset_server.load("sprites/obstacle.png");

    commands
        .spawn(Name::new("Obstacle"))
        .insert(ObstacleParent)
        .insert(RigidBody::Dynamic)
        .insert(LockedAxes::TRANSLATION_LOCKED_Y)
        .insert(Velocity::linear(Vec2::NEG_X * 150.0))
        .insert(Collider::cuboid(32.0, 378.0))
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(Sensor)
        .insert(SpatialBundle::from_transform(Transform::from_translation(
            spawner_transform.translation + Vec3::Y * offset,
        )))
        .with_children(|children| {
            children
                .spawn(Name::new("Obstacle Up"))
                .insert(ObstaclePart)
                .insert(Collider::cuboid(32.0, 128.0))
                .insert(SpriteBundle {
                    texture: obstacle_handle.clone(),
                    transform: Transform::from_translation(Vec3::Y * 250.0),
                    ..default()
                });
            children
                .spawn(Name::new("Score sensor"))
                .insert(ObstacleScore)
                .insert(TransformBundle::default())
                .insert(Collider::cuboid(10.0, 122.0))
                .insert(Sensor);
            children
                .spawn(Name::new("Obstacle Down"))
                .insert(ObstaclePart)
                .insert(Collider::cuboid(32.0, 128.0))
                .insert(SpriteBundle {
                    texture: obstacle_handle.clone(),
                    transform: Transform::from_translation(Vec3::NEG_Y * 250.0),
                    ..default()
                });
        });
}

// Workaround to generic consts only supporting primitive types
trait CollisionVariant {
    const VARIANT: CollisionType;
}

struct Started;
impl CollisionVariant for Started {
    const VARIANT: CollisionType = CollisionType::Started;
}

struct Stopped;
impl CollisionVariant for Stopped {
    const VARIANT: CollisionType = CollisionType::Stopped;
}

enum CollisionType {
    Started,
    Stopped,
}

fn on_collision<T: Component, U: Component, V: CollisionVariant>(
    collision_events: EventReader<CollisionEvent>,
    first_q: Query<Entity, With<T>>,
    second_q: Query<Entity, With<U>>,
) -> bool {
    // this could be optimized with an additional const generic param to break the loop
    // when 1 have been found but I'm leaving it like this for simplicity and ergonomics
    !get_collisions::<T, U, V>(collision_events, first_q, second_q).is_empty()
}

fn get_collisions<T: Component, U: Component, V: CollisionVariant>(
    mut collision_events: EventReader<CollisionEvent>,
    first_q: Query<Entity, With<T>>,
    second_q: Query<Entity, With<U>>,
) -> Vec<(Entity, Entity)> {
    let first_entities: Vec<Entity> = first_q.iter().collect();
    let second_entities: Vec<Entity> = second_q.iter().collect();
    let mut collisions = vec![];

    for collision_event in collision_events.iter() {
        match (collision_event, V::VARIANT) {
            (CollisionEvent::Started(col_1, col_2, _), CollisionType::Started)
            | (CollisionEvent::Stopped(col_1, col_2, _), CollisionType::Stopped) => {
                // let's ensure that the colliding tuple is always (T, U) and not (U, T)
                if first_entities.contains(col_1) && second_entities.contains(col_2) {
                    collisions.push((col_1.clone(), col_2.clone()));
                } else if first_entities.contains(col_2) && second_entities.contains(col_1) {
                    collisions.push((col_2.clone(), col_1.clone()));
                }
            }
            _ => {}
        }
    }

    collisions
}

fn score_up(mut score: ResMut<Score>) {
    **score += 1;
}

fn print_score(score: Res<Score>) {
    println!("Current score: {}", **score);
}

fn game_over(mut commands: Commands, windows_q: Query<(Entity, &Window)>) {
    for (window, _) in windows_q.iter() {
        commands.entity(window).despawn();
    }
}
