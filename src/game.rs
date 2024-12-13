use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use bevy::{
    color::palettes::css::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ui::widget::TextUiWriter,
    window::PresentMode,
};
use bevy_rapier2d::{prelude::*, rapier::geometry::CollisionEventFlags};
use image::{self, GenericImageView};

use std::{collections::VecDeque, time::Duration};

use rand::prelude::*;

pub mod floodfill;

use super::{despawn_screen, GameState};

#[derive(Resource)]
struct Random {
    rng: StdRng,
}

impl Default for Random {
    fn default() -> Self {
        Random {
            rng: StdRng::from_seed([0xDA; 32]),
        }
    }
}

pub fn game_plugin(app: &mut App) {
    use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
    app.add_plugins(FrameTimeDiagnosticsPlugin::default());

    let obj = Objectives {
        time_limit: Some(100),
        load_time: 0.0,
        accepted_missions: vec!["Take a shit".to_string()],
        day: 1,
        map_flags: vec![],
    };

    app.add_plugins((RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),))
        .add_plugins(RapierDebugRenderPlugin::default())
        .init_asset_loader::<MapLoader>()
        .init_asset::<MapAsset>()
        .insert_resource(obj)
        .insert_resource(Random::default())
        .add_systems(Startup, load_map)
        .add_systems(
            OnEnter(GameState::Explore),
            (spawn_player, start_exploration, spawn_ui, debugging_info),
        )
        .add_systems(
            Update,
            (
                player_movement,
                update_camera,
                read_character_controller_collisions,
                change_text_system,
                on_pickup,
                time_pressure,
            )
                .run_if(in_state(GameState::Explore)),
        )
        .add_systems(
            PostUpdate,
            check_triggers.run_if(in_state(GameState::Explore)),
        )
        .insert_resource(Events::<ItemPickupEvent>::default())
        .add_systems(OnExit(GameState::Explore), despawn_screen::<OnExploration>);
}

// The float value is the player movement speed in 'pixels/second'.
#[derive(Component, Default)]
pub struct Player {
    speed: f32,
    velocity: Vec2,
    grounded: bool,
    last_pos: Vec2,
}


#[derive(Resource, Debug,Clone)]
pub struct Objectives {
    time_limit: Option<usize>,
    load_time: f64,
    accepted_missions: Vec<String>,
    day: usize,
    map_flags: Vec<String>
}

#[derive(Component)]
struct OnExploration;

#[derive(Asset, TypePath, Debug)]
pub struct MapAsset {
    // 1000x1000
    pub tiles: Vec<[Tile; 1000]>,
}

#[derive(Component)]
struct LacksCollider;

#[derive(Resource)]
struct ExplorationMap {
    handle: Handle<MapAsset>,
    sprites: TileSprites,
}

impl ExplorationMap {
    fn get_sprite(&self, tile: Tile) -> Sprite {
        match tile {
            Tile::Rock => self.sprites.rock.clone(),
            Tile::Error => self.sprites.error.clone(),
            _ => self.sprites.nothing.clone(),
        }
    }
}

struct TileSprites {
    rock: Sprite,
    nothing: Sprite,
    error: Sprite,
}

#[derive(Default)]
struct MapLoader;

impl MapAsset {
    pub fn from_image(image: &image::DynamicImage) -> MapAsset {
        assert!(image.width() == 1000 && image.height() == 1000);

        let mut tiles = Vec::new();

        for y in 0..1000 {
            let mut next_row = [Tile::Error; 1000];
            for x in 0..1000 {
                let color = image.get_pixel(x, y).0;
                next_row[x as usize] = tile_from_color(color);
            }
            tiles.push(next_row);
        }

        MapAsset { tiles }
    }
}

impl AssetLoader for MapLoader {
    type Asset = MapAsset;
    type Settings = ();
    type Error = anyhow::Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        use std::io::{self, BufRead, Read, Seek};

        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let cursor = io::Cursor::new(&bytes[..]);
        let image = image::ImageReader::new(cursor)
            .with_guessed_format()?
            .decode()?;
        Ok(MapAsset::from_image(&image))
    }

    fn extensions(&self) -> &[&str] {
        &["custom"]
    }
}

fn tile_from_color(color: [u8; 4]) -> Tile {
    match (u32::from_be_bytes(color) & 0xFFFFFF00) >> 8 {
        0xFF_FF_FF | 0x30_30_30 => Tile::Air,
        0xFD_DD_00 => Tile::Rock,
        0x55_cc_ee => Tile::Ice,
        0x00_00_FF => Tile::Oil,
        _ => Tile::Error,
    }
}

fn load_map(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let map: Handle<MapAsset> = asset_server.load("Map/map.png");
    let make_sprite = |image: &str, coord| Sprite {
        image: asset_server.load(image),
        custom_size: Some(Vec2::new(104.0, 104.0)),
        rect: Some(Rect {
            min: coord,
            max: coord + SPRITE_SIZE,
        }),
        ..Default::default()
    };

    const SPRITE_SIZE: Vec2 = Vec2::new(16.0, 16.0);
    const ROCK: Vec2 = Vec2::new(96.0, 48.0);

    let sprites = TileSprites {
        rock: make_sprite("Map/tileset_deeper_and_deeper.png", ROCK),
        nothing: Sprite {
            color: Color::rgba(0.0, 0.0, 0.0, 0.0),
            custom_size: Some(Vec2::new(100.0, 100.0)),
            ..Default::default()
        },
        error: Sprite {
            color: Color::rgb(1.0, 1.0, 0.0),
            custom_size: Some(Vec2::new(100.0, 100.0)),
            ..Default::default()
        },
    };
    commands.insert_resource(ExplorationMap {
        handle: map,
        sprites,
    });
}

#[derive(Component)]
struct TimerHud;

pub fn start_exploration(commands: Commands) {}

#[derive(Copy, Clone, Debug, Default)]
pub enum Tile {
    #[default]
    Error = 0,
    Rock,
    Ice,
    Oil,
    Iron,
    Air,
}

impl Tile {
    fn is_solid(&self) -> bool {
        use Tile as T;
        match self {
            T::Error | T::Rock | T::Ice | T::Oil | T::Iron => true,
            T::Air => false,
        }
    }
}

pub fn spawn_player(
    mut commands: Commands,
    mut rng: ResMut<Random>,
    server: Res<AssetServer>,
    map: Res<ExplorationMap>,
    maps: Res<Assets<MapAsset>>,
    mut rapier_config: Query<&mut RapierConfiguration>,
) {
    let mut rapier_config = rapier_config.single_mut();
    // Set gravity to 0.0 and spawn camera.
    rapier_config.gravity = Vec2::ZERO;

    let sprite_size = 100.0;

    // Spawn entity with `Player` struct as a component for access in movement query.
    commands
        .spawn((
            Sprite {
                custom_size: Some(Vec2::new(sprite_size, sprite_size)),
                image: server.load("mascot.png"),
                image_mode: SpriteImageMode::Auto,
                ..Default::default()
            },
            RigidBody::KinematicPositionBased,
            TransformBundle::from(Transform::from_xyz(200.0, -7000.0, 0.0)),
            Collider::cuboid(sprite_size / 2., sprite_size / 2.0),
            Player {
                grounded: false,
                speed: 10.0,
                velocity: Vec2::ZERO,
                last_pos: Vec2::ZERO,
            },
            OnExploration,
        ))
        .insert(KinematicCharacterController {
            snap_to_ground: Some(CharacterLength::Absolute(0.1)),
            offset: CharacterLength::Absolute(1.0),
            autostep: Some(CharacterAutostep {
                min_width: CharacterLength::Absolute(1.0),
                max_height: CharacterLength::Absolute(1.0),
                include_dynamic_bodies: false,
            }),
            ..Default::default()
        })
        .insert(ActiveCollisionTypes::KINEMATIC_STATIC);

    commands.spawn(Collectable {
        sprite: SpriteBundle {
            sprite: Sprite {
                image: server.load("Portraits/Character_cat.png"),
                custom_size: Some(Vec2::new(sprite_size, sprite_size)),
                ..Default::default()
            },
            transform: Transform::from_xyz(600.0, -7000.0, 0.0),
            ..Default::default()
        },
        collider: Collider::ball(50.0),
        ..Default::default()
    });

    let tiles: &Vec<[Tile; 1000]> = &maps.get(&map.handle).unwrap().tiles;

    let flood = floodfill::floodfill_all(tiles);
    for region in flood.regions {
        let verts = floodfill::get_verts(&region);

        let vertices = verts
            .into_iter()
            .map(|(x, y)| Vec2::new(x as f32 * 100.0 as f32 - 50.0, y as f32 * -100.0 + 50.0))
            .collect::<Vec<Vec2>>();

        commands.spawn((RigidBody::Fixed, Collider::polyline(vertices, None)));
    }

    for (x, row) in tiles.iter().enumerate() {
        for (y, tile) in row.iter().enumerate() {
            let transform =
                Transform::from_xyz(x as f32 * 100.0 - 2.0, -(y as f32 * 100.0 - 2.0), -1.0);

            commands.spawn((
                transform,
                Sprite {
                    flip_x: rng.rng.gen::<bool>(),
                    flip_y: rng.rng.gen::<bool>(),
                    ..map.get_sprite(tiles[y][x])
                },
                OnExploration,
            ));
        }
    }
}

pub fn is_exposed_and_solid(tiles: &Vec<[Tile; 1000]>, x: usize, y: usize) -> bool {
    if !tiles[y][x].is_solid() {
        false
    } else if matches!((x, y), (0 | 999, _) | (_, 0 | 999)) {
        true
    } else {
        if tiles[y][x].is_solid() {
            [
                &tiles[y][x - 1],
                &tiles[y][x + 1],
                &tiles[y - 1][x],
                &tiles[y + 1][x],
            ]
            .iter()
            .any(|tile| !tile.is_solid())
        } else {
            false
        }
    }
}

#[derive(Debug, Event)]
struct ItemPickupEvent {
    item_id: usize,
}

#[derive(Component, Default)]
struct Item {
    id: usize,
}

#[derive(Bundle)]
struct Collectable {
    sprite: SpriteBundle,
    collider: Collider,
    sensor: Sensor,
    item: Item,
    active_events: ActiveEvents,
}

impl Default for Collectable {
    fn default() -> Self {
        Self {
            sprite: default(),
            collider: default(),
            sensor: Sensor,
            item: default(),
            active_events: ActiveEvents::COLLISION_EVENTS,
        }
    }
}

fn on_pickup(mut reader: EventReader<ItemPickupEvent>) {
    for pickup in reader.read() {
        println!("picked up {:?}", pickup);
    }
}

fn check_triggers(
    mut commands: Commands,
    mut reader: EventReader<CollisionEvent>,
    mut writer: EventWriter<ItemPickupEvent>,
    sensors: Query<(Entity, &Item), With<Sensor>>,
) {
    for collision in reader.read() {
        if let CollisionEvent::Started(a, b, flags) = collision {
            if !flags.contains(CollisionEventFlags::SENSOR) {
                continue;
            }
            if let Ok((entity, item)) = sensors.get(*a).or(sensors.get(*b)) {
                writer.send(ItemPickupEvent { item_id: item.id });
                commands.entity(entity).despawn();
            }
        }
    }
}

fn player_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_info: Query<(&mut Player, &mut KinematicCharacterController)>,
) {
    for (mut player, mut controller) in &mut player_info {
        let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
        let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
        let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
        let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

        if up && player.grounded {
            player.velocity += Vec2::new(0.0, 150.0)
        }

        let x_axis = -(left as i8) + right as i8;

        let mut move_delta = Vec2::new(x_axis as f32, 0.0);
        if move_delta != Vec2::ZERO {
            move_delta /= move_delta.length();
        }

        if !player.grounded {
            player.velocity += Vec2::new(0.0, -250.0) * time.delta_secs();
            let drag = player.velocity * 0.02;
            player.velocity -= drag;
        }
        player.velocity = player.velocity.clamp_length_max(300.);

        move_delta += player.velocity * time.delta_secs();

        // Update the velocity on the rigid_body_component,
        // the bevy_rapier plugin will update the Sprite transform.
        controller.translation = Some(move_delta * player.speed);
    }
}

fn update_camera(
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
    player: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Ok(mut camera) = camera.get_single_mut() else {
        return;
    };

    let Ok(player) = player.get_single() else {
        return;
    };

    let Vec3 { x, y, .. } = player.translation;
    let direction = Vec3::new(x, y, camera.translation.z);

    // Applies a smooth effect to camera movement using stable interpolation
    // between the camera position and the player position on the x and y axes.
    camera
        .translation
        .smooth_nudge(&direction, 10.0, time.delta_secs());
}

fn read_character_controller_collisions(
    mut character_controller_outputs: Query<&mut KinematicCharacterControllerOutput>,
    bodies: Query<&RigidBody>,
    mut player: Query<(&mut Player, &mut Transform), With<Player>>,
) {
    let Ok((mut player, mut transform)) = player.get_single_mut() else {
        return;
    };

    let Ok(output) = character_controller_outputs.get_single_mut() else {
        return;
    };

    // we left the ground
    if player.grounded && !output.grounded {
        player.last_pos = transform.translation.xy();
    }

    // we hit the ground
    if !player.grounded && output.grounded {
        if player.velocity.y <= -165.0 {
            transform.translation = player.last_pos.extend(0.0);
        }
        player.velocity.y = player.velocity.y.max(0.0);
    }

    player.grounded = output.grounded;
}

fn spawn_ui(time: Res<Time>,mut commands: Commands, asset_server: ResMut<AssetServer>,mut objective: ResMut<Objectives>) {
    let font = asset_server.load("fonts/Pixelfont/slkscr.ttf");


    let root_uinode = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .id();

    let left_column = commands
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Start,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(15.), Val::Px(5.)),
            ..default()
        },            ))
        .with_children(|builder| {
            builder
                .spawn((
                    Text::default(),
                    TextFont {
                        font: font.clone(),
                        font_size: 40.0,
                        ..default()
                    },
                    TimerHud,
                ))
                .with_children(|p| {
                    p.spawn((
                        TextSpan::default(),
                        TextFont {
                            font: font.clone(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(WHITE.into()),
                    ));
                    p.spawn((
                        TextSpan::default(),
                        TextFont {
                            font: font.clone(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(WHITE.into()),
                    ));
                });
        })
        .id();
    commands.entity(root_uinode).add_children(&[left_column]);
    objective.load_time = time.elapsed().as_secs_f64();
}

fn debugging_info(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    let root_uinode = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .id();

    let right_column = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(15.), Val::Px(5.)),
            ..default()
        })
        .with_children(|builder| {
            builder
                .spawn((
                    Text::default(),
                    TextFont {
                        font: font.clone(),
                        font_size: 21.0,
                        ..default()
                    },
                    TextChanges,
                    BackgroundColor(BLACK.into()),
                ))
                .with_children(|p| {
                    p.spawn((
                        TextSpan::default(),
                        TextFont {
                            font: font.clone(),
                            font_size: 21.0,
                            ..default()
                        },
                        TextColor(WHITE.into()),
                    ));
                    p.spawn((
                        TextSpan::default(),
                        TextFont {
                            font: font.clone(),
                            font_size: 21.0,
                            ..default()
                        },
                        TextColor(WHITE.into()),
                    ));
                });
        })
        .id();
    commands.entity(root_uinode).add_children(&[right_column]);
}

fn time_pressure(time: Res<Time>,mut commands: Commands, asset_server: ResMut<AssetServer>,query: Query<Entity, With<TimerHud>>,
                 objective: ResMut<Objectives>,
                 mut writer: TextUiWriter,) {
    let mut t = time.elapsed().as_secs_f64() - objective.load_time;

    if let Some(timer) = objective.time_limit{
        t = (timer as f64) - t;

        if t < 0.0{
            println!("You have run out of oxygen");
        }
    }
    for entity in &query {
        let display_time = t.max(0.0001);

        *writer.text(entity, 0) =
            format!("{display_time}",);
//        *writer.text(entity, 1) = format!("You are fucked");
    }


}

#[derive(Component)]
struct TextChanges;

fn change_text_system(
    mut fps_history: Local<VecDeque<f64>>,
    mut time_history: Local<VecDeque<Duration>>,
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    query: Query<Entity, With<TextChanges>>,
    player: Query<&Player>,
    mut writer: TextUiWriter,
) {
    time_history.push_front(time.elapsed());
    time_history.truncate(120);
    let avg_fps = (time_history.len() as f64)
        / (time_history.front().copied().unwrap_or_default()
            - time_history.back().copied().unwrap_or_default())
        .as_secs_f64()
        .max(0.0001);
    fps_history.push_front(avg_fps);
    fps_history.truncate(120);
    let fps_variance = std_deviation(fps_history.make_contiguous()).unwrap_or_default();

    for entity in &query {
        let mut fps = 0.0;
        if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps_smoothed) = fps_diagnostic.smoothed() {
                fps = fps_smoothed;
            }
        }

        let mut frame_time = time.delta_secs_f64();
        if let Some(frame_time_diagnostic) =
            diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        {
            if let Some(frame_time_smoothed) = frame_time_diagnostic.smoothed() {
                frame_time = frame_time_smoothed;
            }
        }

        *writer.text(entity, 0) =
            format!("{avg_fps:.1} avg fps, {fps_variance:.1} frametime variance",);
        *writer.text(entity, 1) = format!(
            "\n{:.0} px/s",
            player.get_single().unwrap().velocity.length()
        );
    }
}

fn mean(data: &[f64]) -> Option<f64> {
    let sum = data.iter().sum::<f64>();
    let count = data.len();

    match count {
        positive if positive > 0 => Some(sum / count as f64),
        _ => None,
    }
}

fn std_deviation(data: &[f64]) -> Option<f64> {
    match (mean(data), data.len()) {
        (Some(data_mean), count) if count > 0 => {
            let variance = data
                .iter()
                .map(|value| {
                    let diff = data_mean - *value;

                    diff * diff
                })
                .sum::<f64>()
                / count as f64;

            Some(variance.sqrt())
        }
        _ => None,
    }
}
