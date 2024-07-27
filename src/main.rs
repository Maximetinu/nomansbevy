use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;
use std::ops::Range;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::new(),
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
            RapierDebugRenderPlugin::default(),
        ))
        .insert_resource(Score(0))
        .add_systems(
            Startup,
            (
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
                close.run_if(just_pressed(KeyCode::Escape)),
                close.run_if(on_game_over),
                // small experiment to define jump system scoped under Player impl:
                Player::jump.run_if(just_pressed(KeyCode::Space)),
                tick_spawn_timer,
                spawn_obstacle.run_if(spawn_timer_just_finished),
                get_collisions::<ObstacleRoot, Bounds, Stopped>.pipe(despawn),
                emit_game_over.run_if(on_collision::<Player, ObstaclePart, Started>),
                emit_game_over.run_if(on_collision::<Player, Bounds, Stopped>),
                score_up.run_if(on_collision::<Player, ScoreSensor, Started>),
                print_score.run_if(resource_changed::<Score>),
            ),
        )
        .run();
}

fn despawn(In(entities): In<Vec<Entity>>, mut commands: Commands) {
    for e in entities {
        commands.entity(e).despawn_recursive();
    }
}

fn just_pressed(key_code: KeyCode) -> impl FnMut(Res<ButtonInput<KeyCode>>) -> bool {
    move |input: Res<ButtonInput<KeyCode>>| input.just_pressed(key_code)
}

#[derive(Event)]
struct GameOver;

fn emit_game_over(mut evts: EventWriter<GameOver>) {
    evts.send(GameOver);
}

fn on_game_over(evts: EventReader<GameOver>) -> bool {
    !evts.is_empty()
}

fn close(mut commands: Commands, window_q: Query<Entity, With<Window>>) {
    for window in window_q.iter() {
        commands.entity(window).despawn();
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
struct Player;

impl Player {
    // This way of declaring systems, inside of an impl, allows me to registerd them with Player::jump.
    // Q: Is that Bevy idiomatic ? (i.e. Score::up may convince me, but Score::print does not)
    fn jump(mut player_q: Query<(&Player, &mut Velocity)>) {
        const JUMP_VELOCITY: f32 = 500.0;
        let (_, mut player_rb) = player_q.single_mut();
        player_rb.linvel = Vec2::Y * JUMP_VELOCITY;
    }
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    // dimensions of the player img, 64x64
    const IMG_RES: f32 = 64.0;
    const RADIUS: f32 = IMG_RES / 2.0;

    let player_handle: Handle<Image> = asset_server.load("sprites/player.png");
    commands.spawn((
        Name::new("Player"),
        Player,
        RigidBody::Dynamic,
        GravityScale(2.0),
        LockedAxes::ROTATION_LOCKED,
        Velocity::zero(),
        Collider::ball(RADIUS),
        ActiveEvents::COLLISION_EVENTS,
        SpriteBundle {
            texture: player_handle.clone(),
            transform: Transform::default(),
            ..default()
        },
    ));
}

#[derive(Component)]
struct ObstacleSpawner {
    pub timer: Timer,
    pub range: Range<f32>,
}

fn spawn_obstacle_spawner(mut commands: Commands) {
    commands.spawn((
        Name::new("Obstacle Spawner"),
        ObstacleSpawner {
            timer: Timer::from_seconds(3.0, TimerMode::Repeating),
            range: -250.0..250.0,
        },
        TransformBundle::from(Transform::from_translation(Vec3::X * 700.0)),
    ));
}

fn tick_spawn_timer(time: Res<Time>, mut spawner_q: Query<&mut ObstacleSpawner>) {
    spawner_q.single_mut().timer.tick(time.delta());
}

fn spawn_timer_just_finished(spawner_q: Query<&ObstacleSpawner>) -> bool {
    spawner_q.single().timer.just_finished()
}

#[derive(Component)]
struct Bounds;

fn spawn_bounds(mut commands: Commands) {
    commands.spawn((
        Name::new("Bounds"),
        TransformBundle::default(),
        Bounds,
        Collider::cuboid(700.0, 400.0),
        Sensor,
    ));
}

#[derive(Resource, Deref, DerefMut)]
struct Score(u8); // yes, u8; if player reaches 255, he beats the game

fn score_up(mut score: ResMut<Score>) {
    **score += 1;
}

fn print_score(score: Res<Score>) {
    println!("Current score: {}", **score);
}

#[derive(Component)]
struct ObstaclePart;

#[derive(Component)]
struct ObstacleRoot;

#[derive(Component)]
struct ScoreSensor;

fn spawn_obstacle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    spawner_q: Query<(&Transform, &ObstacleSpawner)>,
) {
    // dimensions of obstacle.png image
    const PART_X: f32 = 64.0;
    const PART_Y: f32 = 512.0;

    // arbitrary gap, smaller means harder for the player
    const GAP_HEIGHT: f32 = 140.0;

    // calculations
    const PART_DISPLACEMENT: f32 = GAP_HEIGHT / 2.0 + PART_Y / 2.0;
    const PART_WIDTH: f32 = PART_X / 2.0;
    const PART_HEIGHT: f32 = PART_Y / 2.0;
    const SCORE_WIDTH: f32 = 2.0; // arbitrary, super thin is enough to detect the player
    const SCORE_HEIGHT: f32 = GAP_HEIGHT / 2.0;
    const TOTAL_HEIGHT: f32 = PART_HEIGHT * 2.0 + SCORE_HEIGHT;

    // greater is harder for the player
    const SPEED: f32 = 150.0;

    let (spawner_transform, spawner) = spawner_q.single();

    let offset = thread_rng().gen_range(spawner.range.clone());

    let obstacle_handle: Handle<Image> = asset_server.load("sprites/obstacle.png");

    commands
        .spawn((
            Name::new("Obstacle"),
            ObstacleRoot,
            RigidBody::Dynamic,
            LockedAxes::TRANSLATION_LOCKED_Y,
            Velocity::linear(Vec2::NEG_X * SPEED),
            Collider::cuboid(PART_WIDTH, TOTAL_HEIGHT),
            ActiveEvents::COLLISION_EVENTS,
            Sensor,
            SpatialBundle::from_transform(Transform::from_translation(
                spawner_transform.translation + Vec3::Y * offset,
            )),
        ))
        .with_children(|children| {
            children.spawn((
                Name::new("Obstacle Up"),
                ObstaclePart,
                Collider::cuboid(PART_WIDTH, PART_HEIGHT),
                SpriteBundle {
                    texture: obstacle_handle.clone(),
                    transform: Transform::from_translation(Vec3::Y * PART_DISPLACEMENT),
                    ..default()
                },
            ));
            children.spawn((
                Name::new("Score sensor"),
                ScoreSensor,
                TransformBundle::default(),
                Collider::cuboid(SCORE_WIDTH, SCORE_HEIGHT),
                Sensor,
            ));
            children.spawn((
                Name::new("Obstacle Down"),
                ObstaclePart,
                Collider::cuboid(PART_WIDTH, PART_HEIGHT),
                SpriteBundle {
                    texture: obstacle_handle.clone(),
                    transform: Transform::from_translation(Vec3::NEG_Y * PART_DISPLACEMENT),
                    ..default()
                },
            ));
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

// Alternatively, a more complex but powerful version
// could return colliding pairs, i.e. Vec<(Entity, Entity)>
fn get_collisions<T: Component, U: Component, V: CollisionVariant>(
    mut collision_events: EventReader<CollisionEvent>,
    first_q: Query<Entity, With<T>>,
    second_q: Query<Entity, With<U>>,
) -> Vec<Entity> {
    let first_entities: Vec<Entity> = first_q.iter().collect();
    let second_entities: Vec<Entity> = second_q.iter().collect();
    let mut collisions = vec![];

    for collision_event in collision_events.read() {
        match (collision_event, V::VARIANT) {
            (CollisionEvent::Started(col_1, col_2, _), CollisionType::Started)
            | (CollisionEvent::Stopped(col_1, col_2, _), CollisionType::Stopped) => {
                // let's ensure that the colliding tuple is always (T, U) and not (U, T)
                if first_entities.contains(&col_1) && second_entities.contains(&col_2) {
                    collisions.push(col_1.clone());
                } else if first_entities.contains(&col_2) && second_entities.contains(&col_1) {
                    collisions.push(col_2.clone());
                }
            }
            _ => {}
        }
    }

    collisions
}
