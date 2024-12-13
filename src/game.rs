use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use bevy::{
    color::palettes::css::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::common_conditions::input_just_pressed,
    ui::widget::TextUiWriter,
};
use bevy_rapier2d::{prelude::*, rapier::geometry::CollisionEventFlags};
use image::{self, GenericImageView};

use std::{collections::VecDeque, time::Duration};

use rand::prelude::*;

pub mod floodfill;
use crate::dating_sim;

use self::floodfill::Floodfill;

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
        time_limit: Some(5),
        load_time: 0.0,
        accepted_missions: vec!["Take a shit".to_string()],
        day: 1,
        map_flags: vec![],
    };

    app.add_plugins((RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),))
        // .add_plugins(RapierDebugRenderPlugin::default())
        .init_asset_loader::<MapLoader>()
        .init_asset::<MapAsset>()
        .insert_resource(obj)
        .insert_resource(Random::default())
        .add_systems(Startup, load_map)
        .add_systems(
            OnEnter(GameState::Explore),
            (
                spawn_player,
                start_exploration,
                spawn_ui,
                debugging_info,
                create_spaceship,
            ),
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
                execute_animations,
            )
                .run_if(in_state(GameState::Explore)),
        )
        .add_systems(
            PostUpdate,
            check_triggers.run_if(in_state(GameState::Explore)),
        )
        .insert_resource(Events::<WorldTriggerEvent>::default())
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

#[derive(Resource, Debug, Clone)]
pub struct Objectives {
    time_limit: Option<usize>,
    load_time: f64,
    accepted_missions: Vec<String>,
    day: usize,
    map_flags: Vec<String>,
}

#[derive(Component)]
struct OnExploration;

#[derive(Asset, TypePath)]
pub struct MapAsset {
    // 1000x1000
    pub tiles: Vec<[Tile; 1000]>,
    floodfill: Floodfill,
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
            Tile::Error => self.sprites.nothing.clone(),
            Tile::Wall => self.sprites.backdrop.clone(),
            _ => self.sprites.nothing.clone(),
        }
    }
}

struct TileSprites {
    rock: Sprite,
    nothing: Sprite,
    backdrop: Sprite,
    sodium: Sprite,
    calcium: Sprite,
    coal: Sprite,
    iron: Sprite,
    potassium: Sprite,
    sulfur: Sprite,
    oil: Sprite,
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

        let floodfill = floodfill::floodfill_all(&tiles);

        MapAsset { tiles, floodfill }
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
        use std::io;

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
        0xFF_FF_FF => Tile::Air,
        0x30_30_30 => Tile::Wall,
        0xFD_DD_00 => Tile::Rock,
        0x55_cc_ee => Tile::Ice,
        0x00_00_FF => Tile::Oil,
        0xFF_00_00 => Tile::Sulfur,
        0x77_44_ff => Tile::Coal,
        _ => Tile::Error,
    }
}

fn load_map(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let map: Handle<MapAsset> = asset_server.load("Map/map.png");
    let make_sprite = |image: &str, coord| Sprite {
        image: asset_server.load(image),
        custom_size: Some(Vec2::new(100.5, 100.5)),
        rect: Some(Rect {
            min: coord,
            max: coord + SPRITE_SIZE,
        }),
        ..Default::default()
    };

    const SPRITE_SIZE: Vec2 = Vec2::new(16.0, 16.0);
    const ROCK: Vec2 = Vec2::new(16.0, 16.0);
    const BACKDROP: Vec2 = Vec2::new(96.0, 48.0);

    let backdrop = Sprite {
        color: Color::srgba(0.4, 0.4, 0.4, 1.0),
        ..make_sprite("Map/tileset_deeper_and_deeper.png", ROCK)
    };

    let sprites = TileSprites {
        rock: make_sprite("Map/tileset_deeper_and_deeper.png", ROCK),
        backdrop,
        sodium: make_sprite("Map/tileset_deeper_and_deeper.png", Vec2::new(144.0, 0.0)),
        calcium: make_sprite(
            "Map/tileset_deeper_and_deeper.png",
            Vec2::new(144.0 + 16.0, 0.0),
        ),
        coal: make_sprite("Map/tileset_deeper_and_deeper.png", Vec2::new(144.0, 16.0)),
        iron: make_sprite(
            "Map/tileset_deeper_and_deeper.png",
            Vec2::new(144.0 + 46.0, 0.0),
        ),
        potassium: make_sprite("Map/tileset_deeper_and_deeper.png", Vec2::new(160.0, 16.0)),
        sulfur: make_sprite("Map/tileset_deeper_and_deeper.png", Vec2::new(176.0, 0.0)),
        oil: make_sprite("Map/tileset_deeper_and_deeper.png", Vec2::new(176.0, 16.0)),
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
    Wall,
    Sulfur,
    Coal,
}

impl Tile {
    fn is_solid(&self) -> bool {
        use Tile as T;
        match self {
            T::Error | T::Sulfur | T::Coal | T::Rock | T::Ice | T::Oil | T::Iron => true,
            T::Air | T::Wall => false,
        }
    }
}

#[derive(Component)]
struct OutsideOST;

#[derive(Component)]
struct BackgroundExplore;

fn pick_ost(index: usize) -> String {
    let path = match index {
        1 => "Music/Music_CaveTheme1.ogg".to_string(),
        2 => "Music/Music_CaveTheme2.ogg".to_string(),
        3 => "Music/Music_OutdoorTheme1.ogg".to_string(),
        _ => "Music/Music_OutdoorTheme2.ogg".to_string(),
    };
    path
}

fn spawn_player(
    mut commands: Commands,
    mut rng: ResMut<Random>,
    server: Res<AssetServer>,
    map: Res<ExplorationMap>,
    maps: Res<Assets<MapAsset>>,
    mut rapier_config: Query<&mut RapierConfiguration>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    bg: Option<Single<Entity, (With<Transform>, With<BackgroundExplore>)>>,
    windows: Query<&mut Window>,
) {
    log::info!("spawning player");

    let mut rapier_config = rapier_config.single_mut();
    // Set gravity to 0.0 and spawn camera.
    rapier_config.gravity = Vec2::ZERO;

    let sprite_size = 100.0;

    // commands.spawn((
    //     AudioPlayer::new(server.load("sounds/hev charger drip car.mp3")),
    //     OutsideOST,
    // ));

    let width = windows.single().resolution.width();
    let height = windows.single().resolution.height();

    let background_size = Some(Vec2::new(width, height));
    let background_position = Transform::from_xyz(0.0, 0.0, -1000.0);

    let texture = server.load("Backgrounds/deeper_deeper_galaxy.png");
    // the sprite sheet has 7 sprites arranged in a row, and they are all 24px x 24px
    let layout = TextureAtlasLayout::from_grid(UVec2::new(1000, 600), 5, 4, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let mut animation_config_1 = AnimationConfig::new(0, 1, 16, 10);
    animation_config_1.running = true;

    if bg.is_none() {
        commands.spawn((
            Sprite {
                image: texture.clone(),
                custom_size: background_size,
                texture_atlas: Some(TextureAtlas {
                    layout: texture_atlas_layout.clone(),
                    index: animation_config_1.first_sprite_index,
                }),
                ..Default::default()
            },
            animation_config_1,
            background_position,
            BackgroundExplore,
        ));
    }

    let random_number = rng.rng.gen_range(0..5);
    commands.spawn((
        AudioPlayer::new(server.load(pick_ost(random_number))),
        OnExploration,
    ));

    let texture = server.load("Sprite/Player_Walking_Sprite-Sheet.png");
    // the sprite sheet has 7 sprites arranged in a row, and they are all 24px x 24px
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(16), 4, 1, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let animation_config_1 = AnimationConfig::new(0, 0, 3, 10);

    // player init
    commands
        .spawn((
            Sprite {
                custom_size: Some(Vec2::new(sprite_size, sprite_size)),
                image: texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: texture_atlas_layout.clone(),
                    index: animation_config_1.first_sprite_index,
                }),
                image_mode: SpriteImageMode::Auto,
                ..Default::default()
            },
            RigidBody::KinematicPositionBased,
            TransformBundle::from(Transform::from_xyz(6800.0, -7500.0, 0.0)),
            Collider::ball(sprite_size / 2.4),
            Player {
                grounded: false,
                speed: 10.0,
                velocity: Vec2::ZERO,
                last_pos: Vec2::new(200.0, -7000.0),
            },
            animation_config_1,
            OnExploration,
        ))
        .insert(KinematicCharacterController {
            snap_to_ground: Some(CharacterLength::Absolute(10.0)),
            offset: CharacterLength::Absolute(2.0),
            autostep: Some(CharacterAutostep {
                min_width: CharacterLength::Absolute(1.0),
                max_height: CharacterLength::Absolute(1.0),
                include_dynamic_bodies: false,
            }),
            ..Default::default()
        })
        .insert(ActiveCollisionTypes::KINEMATIC_STATIC);

    //Place all triggers
    {
        let generous_trigger = Some(Vec2::new(500.0, 500.0));

        //Greenhouse
        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(28000.0, -3400.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            trigger: TriggerComponent {
                id: TriggerType::ItemPickup,
                delete_on_trigger: true,
                flags: Some("GreenhouseFixed".to_string()),
            },
            ..default()
        });

        //coal TODO
        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(3600.0, -7000.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            trigger: TriggerComponent {
                id: TriggerType::ItemPickup,
                delete_on_trigger: true,
                flags: Some("CoalCollected".to_string()),
            },
            ..default()
        });
        //sodium: Sprite TODO

        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(47400.0, -20900.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            trigger: TriggerComponent {
                id: TriggerType::ItemPickup,
                delete_on_trigger: true,
                flags: Some("SodiumCollected".to_string()),
            },
            ..default()
        });

        //calcium: Sprite TODO

        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(42100.0, -13100.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            trigger: TriggerComponent {
                id: TriggerType::ItemPickup,
                delete_on_trigger: true,
                flags: Some("CalciumCollected".to_string()),
            },
            ..default()
        });
        //sulfur
        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(23100.0, -20100.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            trigger: TriggerComponent {
                id: TriggerType::ItemPickup,
                delete_on_trigger: true,
                flags: Some("SulfurCollected".to_string()),
            },
            ..default()
        });
        //potassium TODO
        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(3600.0, -7000.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            ..default()
        });

        //iron
        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(274.0 * 100.0, -11600.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            ..default()
        });

        //oil
        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(39.0 * 100.0, -14100.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            trigger: TriggerComponent {
                id: TriggerType::ItemPickup,
                delete_on_trigger: true,
                flags: Some("OilCollected".to_string()),
            },
            collider: Collider::ball(250.0),
            ..default()
        });

        //beacon TODO
        commands.spawn(WorldTrigger {
            transform: Transform::from_xyz(39.0 * 100.0, -14100.0, 0.0),
            sprite: Sprite {
                image: server.load("Sprite/SpaceBox_Sprite.png"),
                custom_size: generous_trigger,
                ..default()
            },
            ..default()
        });
    }

    // the second (right-hand) sprite runs at 20 FPS
    let MapAsset {
        ref tiles,
        floodfill: ref flood,
    } = &maps.get(&map.handle).unwrap();

    let flood = floodfill::floodfill_all(tiles);
    for region in flood.regions {
        let verts = floodfill::get_verts(&region);

        let vertices = verts
            .into_iter()
            .map(|(x, y)| Vec2::new(x as f32 * 100.0 as f32 - 50.0, y as f32 * -100.0 + 50.0))
            .collect::<Vec<Vec2>>();

        commands.spawn((
            RigidBody::Fixed,
            Collider::polyline(vertices, None),
            OnExploration,
        ));
    }

    let mut sprites = Vec::with_capacity(1000 * 1000);

    for (x, row) in tiles.iter().enumerate() {
        for (y, tile) in row.iter().enumerate() {
            let transform = Transform::from_xyz(x as f32 * 100.0, -(y as f32 * 100.0), -1.0);

            sprites.push((
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

    commands.spawn_batch(sprites);
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

fn create_spaceship(mut commands: Commands, server: ResMut<AssetServer>) {
    // 65, 75

    commands.spawn(WorldTrigger {
        sprite: Sprite {
            image: server.load("Sprite/SpacShip_Sprite.png"),
            custom_size: Some(Vec2::new(1600.0, 800.0)),
            ..default()
        },
        transform: Transform::from_xyz(6000.0, -7400.0, 0.0),
        collider: Collider::cuboid(150.0, 150.0),
        trigger: TriggerComponent {
            id: TriggerType::Ship,
            delete_on_trigger: false,
            flags: None,
        },
        ..default()
    });
}

#[derive(Debug, Event)]
struct WorldTriggerEvent {
    trigger_type: TriggerType,
    message: (),
    flag: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
enum TriggerType {
    #[default]
    None,
    ItemPickup,
    Ship,
}

#[derive(Component, Default)]
struct TriggerComponent {
    id: TriggerType,
    delete_on_trigger: bool,
    flags: Option<String>,
}

#[derive(Bundle)]
struct WorldTrigger {
    sprite: Sprite,
    transform: Transform,
    collider: Collider,
    sensor: Sensor,
    trigger: TriggerComponent,
    active_events: ActiveEvents,
}

impl Default for WorldTrigger {
    fn default() -> Self {
        Self {
            sprite: default(),
            transform: default(),
            collider: default(),
            sensor: Sensor,
            active_events: ActiveEvents::COLLISION_EVENTS,
            trigger: TriggerComponent {
                id: TriggerType::None,
                delete_on_trigger: true,
                flags: None,
            },
        }
    }
}

fn on_pickup(
    mut reader: EventReader<WorldTriggerEvent>,
    mut context: ResMut<dating_sim::DatingContext>,
    mut commands: Commands,
    server: Res<AssetServer>,
) {
    for event in reader.read() {
        if event.trigger_type != TriggerType::ItemPickup {
            continue;
        } else {
            commands.spawn(AudioPlayer::new(server.load("sounds/Pickup.ogg")));
            if let Some(key) = &event.flag {
                let flag = context.flags.entry(key.clone()).or_insert(0);
                *flag += 1;
            }
        }
    }
}

fn check_triggers(
    mut commands: Commands,
    mut reader: EventReader<CollisionEvent>,
    mut writer: EventWriter<WorldTriggerEvent>,
    sensors: Query<(Entity, &TriggerComponent), With<Sensor>>,
) {
    for collision in reader.read() {
        if let CollisionEvent::Started(a, b, flags) = collision {
            if !flags.contains(CollisionEventFlags::SENSOR) {
                continue;
            }
            if let Ok((entity, trigger)) = sensors.get(*a).or(sensors.get(*b)) {
                writer.send(WorldTriggerEvent {
                    trigger_type: trigger.id,
                    message: (),
                    flag: trigger.flags.clone(),
                });
                if trigger.delete_on_trigger {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn player_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut jump_buffer: Local<usize>,
    mut player_info: Query<(
        &mut Player,
        &mut Sprite,
        &mut AnimationConfig,
        &mut KinematicCharacterController,
    )>,
) {
    *jump_buffer = jump_buffer.saturating_sub(1);

    for (mut player, mut sprite, mut animation, mut controller) in &mut player_info {
        let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
        let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
        let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
        let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
        if keyboard_input.just_pressed(KeyCode::KeyW) && !player.grounded {
            *jump_buffer = 6;
        }

        animation.running = player.grounded && (left || right);

        if left && !right {
            sprite.flip_x = true;
        } else if right && !left {
            sprite.flip_x = false;
        }

        // jump
        if (keyboard_input.just_pressed(KeyCode::KeyW) || *jump_buffer > 0) && player.grounded {
            player.velocity += Vec2::new(0.0, 100.0);
            player.grounded = false;
            *jump_buffer = 0;
        }

        let x_axis = -(left as i8) + right as i8;

        let mut move_delta = Vec2::new(x_axis as f32, 0.0);
        if move_delta != Vec2::ZERO {
            move_delta /= move_delta.length();
        }

        if !player.grounded {
            player.velocity += Vec2::new(0.0, -200.0) * time.delta_secs();
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
    mut camera: Query<
        &mut Transform,
        (With<Camera2d>, Without<Player>, Without<BackgroundExplore>),
    >,
    player: Query<&Transform, (With<Player>, Without<Camera2d>, Without<BackgroundExplore>)>,
    time: Res<Time>,
    mut background: Single<&mut Transform, (With<BackgroundExplore>)>,
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
    background.translation.x = camera.translation.x;
    background.translation.y = camera.translation.y;
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
        player.velocity.y = player.velocity.y.max(-0.1);
    }

    player.grounded = output.grounded;
}

fn spawn_ui(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut objective: ResMut<Objectives>,
) {
    let font = asset_server.load("fonts/Pixelfont/slkscr.ttf");

    let root_uinode = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .insert(OnExploration)
        .id();

    let left_column = commands
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Start,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(15.), Val::Px(5.)),
            ..default()
        },))
        .insert(OnExploration)
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

fn time_pressure(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    query: Query<Entity, With<TimerHud>>,
    objective: ResMut<Objectives>,
    mut writer: TextUiWriter,
    mut menu_state: ResMut<NextState<GameState>>,
) {
    let mut t = time.elapsed().as_secs_f64() - objective.load_time;

    if let Some(timer) = objective.time_limit {
        t = (timer as f64) - t;

        if t < 0.0 {
            if t < -5.0 {
                println!("You have run out of oxygen");
                menu_state.set(GameState::DatingSim);
            }

            for entity in &query {
                let display_time = t;

                *writer.text(entity, 1) = format!("You must return now!",);
            }
        } else {
            for entity in &query {
                let display_time = t;

                *writer.text(entity, 0) = format!("Oxygen time: {display_time:.0}\n",);
                //        *writer.text(entity, 1) = format!("You are fucked");
            }
        }
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

fn execute_animations(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut AnimationConfig, &mut Sprite)>,
) {
    for (mut config, mut sprite) in &mut query {
        if config.running {
            config.timer.tick(time.delta());

            if config.timer.just_finished() {
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = if atlas.index == config.last_sprite_index {
                        config.first_sprite_index
                    } else {
                        atlas.index + 1
                    };
                }
            }
        } else {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = config.idle_index;
            }
        }
    }
}

#[derive(Component, Debug)]
struct AnimationConfig {
    idle_index: usize,
    first_sprite_index: usize,
    last_sprite_index: usize,
    fps: u8,
    timer: Timer,
    running: bool,
}

impl AnimationConfig {
    fn new(idle: usize, first: usize, last: usize, fps: u8) -> Self {
        Self {
            idle_index: idle,
            first_sprite_index: first,
            last_sprite_index: last,
            fps,
            timer: Self::timer_from_fps(fps),
            running: false,
        }
    }

    fn timer_from_fps(fps: u8) -> Timer {
        Timer::new(
            Duration::from_secs_f32(1.0 / (fps as f32)),
            TimerMode::Repeating,
        )
    }
}
