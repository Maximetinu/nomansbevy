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
        .add_systems(
            Update,
            (
                bevy::window::close_on_esc,
                jump.run_if(just_pressed(KeyCode::Space)),
                r#move,
            ),
        )
        .add_systems(
            PostUpdate,
            (
                handle_player_obstacle_collision
                    .run_if(on_collision_enter::<Player, ObstacleCollider>),
                handle_player_score_collision.run_if(on_collision_enter::<Player, ScoreSensor>),
            ),
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

    let obstacle_handle: Handle<Image> = asset_server.load("sprites/obstacle.png");

    commands
        .spawn(Name::new("Obstacle"))
        .insert(Movement {
            velocity: Vec2::NEG_X * 50.0, // px/s
        })
        .insert(TransformBundle::default())
        .insert(VisibilityBundle::default())
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

#[derive(Component)]
struct Player;

#[derive(Component)]
struct ObstacleCollider;

#[derive(Component)]
struct ScoreSensor;

#[derive(Component)]
struct Movement {
    pub velocity: Vec2,
}

fn jump(mut player_q: Query<(&Player, &mut Velocity)>) {
    const JUMP_VELOCITY: f32 = 400.0;
    let (_, mut player_rb) = player_q.single_mut();
    player_rb.linvel = Vec2::Y * JUMP_VELOCITY;
}

// Artificial perpetual linear movement. We could replace this by a Velocity + Rigidbody
fn r#move(mut moving_q: Query<(&Movement, &mut Transform)>, time: Res<Time>) {
    for (movement, mut transform) in moving_q.iter_mut() {
        transform.translation += movement.velocity.extend(0.0) * time.delta_seconds();
    }
}

fn just_pressed(key_code: KeyCode) -> impl FnMut(Res<Input<KeyCode>>) -> bool {
    move |input: Res<Input<KeyCode>>| input.just_pressed(key_code)
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

fn handle_player_score_collision() {
    println!(">> Player entered an score sensor just now");
}
