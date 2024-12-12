use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use bevy_rapier2d::prelude::*;
use image::{self, GenericImageView};

pub mod floodfill;

use super::{GameState, despawn_screen};

pub fn game_plugin(app: &mut App) {
    {
        use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ));
    }

    app.add_plugins((RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),))
        .add_plugins(RapierDebugRenderPlugin::default())
        .init_asset_loader::<MapLoader>()
        .init_asset::<MapAsset>()
        .add_systems(Startup, load_map)
        .add_systems(
            OnEnter(GameState::Explore),
            (spawn_player, start_exploration),
        )
        .add_systems(
            Update,
            (
                player_movement,
                update_camera,
                read_character_controller_collisions,
            )
                .run_if(in_state(GameState::Explore)),
        )
        .add_systems(OnExit(GameState::Explore), despawn_screen::<OnExploration>);
}

// The float value is the player movement speed in 'pixels/second'.
#[derive(Component)]
pub struct Player {
    speed: f32,
    velocity: Vec2,
    grounded: bool,
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
        0xFF_FF_FF => Tile::Air,
        0xDD_DD_DD => Tile::Rock,
        0x00_00_FF => Tile::Ice,
        _ => Tile::Error,
    }
}

fn load_map(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let map: Handle<MapAsset> = asset_server.load("Map/map.png");
    let make_sprite = |image: &str| Sprite {
        image: asset_server.load(image),
        custom_size: Some(Vec2::new(100.0, 100.0)),
        ..Default::default()
    };

    let sprites = TileSprites {
        rock: make_sprite("Map/rock.png"),
        nothing: Sprite {
            color: Color::WHITE,
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
            TransformBundle::from(Transform::from_xyz(200.0, 200.0, 0.0)),
            Collider::cuboid(sprite_size / 2., sprite_size / 2.0),
            Player {
                grounded: false,
                speed: 10.0,
                velocity: Vec2::ZERO,
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
            let transform = Transform::from_xyz(x as f32 * 100.0, -(y as f32 * 100.0), -1.0);

            commands.spawn((transform, map.get_sprite(tiles[y][x]), OnExploration));
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
            player.velocity += Vec2::new(0.0, 100.0)
        }

        let x_axis = -(left as i8) + right as i8;

        let mut move_delta = Vec2::new(x_axis as f32, 0.0);
        if move_delta != Vec2::ZERO {
            move_delta /= move_delta.length();
        }

        if !player.grounded {
            player.velocity += Vec2::new(0.0, -400.0) * time.delta_secs();
        }
        player.velocity = player.velocity.clamp_length_max(100.);

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
        .smooth_nudge(&direction, 2.0, time.delta_secs());
}

fn read_character_controller_collisions(
    mut character_controller_outputs: Query<&mut KinematicCharacterControllerOutput>,
    bodies: Query<&RigidBody>,
    mut player: Query<(&mut Player, &mut Transform), With<Player>>,
) {
    let Ok((mut player, _transform)) = player.get_single_mut() else {
        return;
    };

    let Ok(output) = character_controller_outputs.get_single_mut() else {
        return;
    };

    player.grounded = output.grounded;
    if player.grounded {
        player.velocity.y = player.velocity.y.max(0.0);
    }
}
