//enum state {
//    talking: usize,
//    picking: Vec<option>,
//}

use super::{despawn_screen, GameState};
use crate::load;
use bevy::{
    math::ops,
    prelude::*,
    text::{FontSmoothing, LineBreak, TextBounds},
    window::PrimaryWindow,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Copy, Clone, Debug)]
enum MissionType {
    Water,
    Explore,
    Oil,
    Iron,
    Tutorial,
}

#[derive(Deserialize, Debug, Copy, Clone)]
enum CharactersType {
    Joe,
    Jule,
    Carle,
    Fredrick,
    Diedrick,
    Cat,
    Liv,
    You,
}

struct CharactersStatus {
    character: CharactersType,
    current_dialogue: String,
}

#[derive(Resource)]
pub struct DatingContext {
    all_characters: Vec<CharactersStatus>,
    day: usize,
    cursor: isize,
    selected_scene: DatingScene,
    pub flags: HashMap<String, isize>,
    gathered_mission: Vec<MissionType>,
    scenes: Vec<DatingScene>,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum DatingState {
    #[default]
    Noting,
    Chilling,
    Talking,
    Choosing,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DatingScene {
    id: SceneID,
    text: Vec<(Option<CharactersType>, String)>,
    outcome: Option<Vec<(Flag, isize)>>,
    choice: Option<Vec<(String, String)>>,
    mission: Option<MissionType>,
    #[serde(default, rename = "scene")]
    next_scene: Vec<(Cond, SceneID)>,
    #[serde(default)]
    black: bool,
}

type Check = (Option<Flag>, isize);
type SceneID = String;
type Flag = String;
type Cond = Vec<Check>;

#[derive(Component)]
struct FollowsMouse;

#[derive(Component)]
struct Cursor(isize);

#[derive(Component)]
struct Portrait;

#[derive(Component)]
struct MissionNot;

#[derive(Component)]
struct DatingObj;

#[derive(Component)]
struct TalkObj;

#[derive(Component)]
struct TextBox(usize);

#[derive(Component)]
struct NameBox;

#[derive(Component)]
struct PortraitBox;

#[derive(Component)]
struct DatingOption;

#[derive(Component)]
struct ChoiceObj(String);

pub fn dating_sim_plugin(app: &mut App) {
    let all_scenes = load::load_scenes();

    let janitor_joe = CharactersStatus {
        character: CharactersType::Joe,
        current_dialogue: "JoeInit".to_string(),
    };

    let cat = CharactersStatus {
        character: CharactersType::Cat,
        current_dialogue: "CatInit".to_string(),
    };

    let granny = CharactersStatus {
        character: CharactersType::Jule,
        current_dialogue: "JuleInit".to_string(),
    };

    let twin1 = CharactersStatus {
        character: CharactersType::Fredrick,
        current_dialogue: "FredrickInit".to_string(),
    };

    let twin2 = CharactersStatus {
        character: CharactersType::Diedrick,
        current_dialogue: "DiedrickInit".to_string(),
    };

    let carly = CharactersStatus {
        character: CharactersType::Carle,
        current_dialogue: "CarleInit".to_string(),
    };

    let liv = CharactersStatus {
        character: CharactersType::Liv,
        current_dialogue: "LivInit".to_string(),
    };

    let characters = vec![janitor_joe, granny, cat, twin1, twin2, carly, liv];

    let first_scene = all_scenes[0].clone();

    let mut initial_events: HashMap<String, isize> = HashMap::new();
    initial_events.insert("day".to_string(), 1);

    app.insert_resource(DatingContext {
        all_characters: characters,
        day: 1,
        cursor: 0,
        selected_scene: first_scene,
        flags: initial_events.clone(),
        gathered_mission: vec![],
        scenes: all_scenes,
    });

    app.init_state::<DatingState>();

    //genereric
    app.add_systems(OnEnter(GameState::DatingSim), on_dating_sim)
        .add_systems(OnExit(GameState::DatingSim), despawn_screen::<DatingObj>);

    //Chilling
    app.add_systems(OnEnter(DatingState::Chilling), on_chill)
        .add_systems(
            Update,
            cursor_action.run_if(in_state(DatingState::Chilling)),
        );

    app.add_systems(OnEnter(DatingState::Choosing), on_choosing)
        .add_systems(Update, choose_move.run_if(in_state(DatingState::Choosing)))
        .add_systems(
            OnExit(DatingState::Choosing),
            (despawn_screen::<ChoiceObj>, despawn_screen::<Cursor>),
        );

    //Choices
    app.add_systems(OnEnter(DatingState::Talking), start_talking)
        .add_systems(
            Update,
            talking_action.run_if(in_state(DatingState::Talking)),
        )
        .add_systems(OnExit(DatingState::Talking), despawn_screen::<TalkObj>);

    app.add_systems(
        OnExit(DatingState::Chilling),
        (despawn_screen::<Portrait>, despawn_screen::<MissionNot>),
    );
}

fn on_dating_sim(
    mut commands: Commands,
    mut tmp: ResMut<NextState<DatingState>>,
    mut did_init: Local<bool>,
    mut context: ResMut<DatingContext>,
    asset_server: Res<AssetServer>,
    camera: Single<
        Entity,
        (
            With<Camera2d>,
            With<Transform>,
            Without<crate::game::Player>,
        ),
    >,
) {
    //    let key = "day".to_string();
    //    if let Some(flag) = context.flags.get_mut(&key) {
    //        *flag += 1;
    //    }
    let entity = camera.into_inner();

    commands.entity(entity).despawn();
    commands.spawn(Camera2d).insert(Transform::default());

    commands.spawn((
        AudioPlayer::new(asset_server.load("Music/Music_InShip.ogg")),
        DatingObj,
    ));
    if !*did_init {
        context.selected_scene = context
            .scenes
            .iter()
            .find(|s: &&DatingScene| s.id == "Day1Morning")
            .cloned()
            .unwrap();
        *did_init = true;
        tmp.set(DatingState::Talking);
    } else {
        tmp.set(DatingState::Chilling);
    }
}

#[derive(Component)]
struct MyOST;

#[derive(Component)]
struct Background;

fn on_chill(
    mut commands: Commands,
    context: ResMut<DatingContext>,
    asset_server: Res<AssetServer>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
    background: Option<Single<&mut Sprite, With<Background>>>,
) {
    let window = windows.single();
    let width = window.resolution.width();
    let height = window.resolution.height();

    let font = asset_server.load("fonts/Pixelfont/slkscr.ttf");
    let text_font = TextFont {
        font: font.clone(),
        font_size: 50.0,
        ..default()
    };

    let slightly_smaller_text_font = TextFont {
        font,
        font_size: 27.0,
        ..default()
    };

    //Cursor initialisation

    if let Some(mut background) = background.map(Single::into_inner) {
        background.color = Color::srgba(1.0, 1.0, 1.0, 1.0);
    } else {
        let background_size = Some(Vec2::new(width, height));
        let background_position = Vec2::new(0.0, 0.0);
        commands.spawn((
            dbg!(Sprite {
                image: asset_server.load("Backgrounds/deeper_deeper_base.png"),
                custom_size: background_size,
                ..Default::default()
            }),
            Transform::from_translation(background_position.extend(-1.0)),
            Background,
            DatingObj,
        ));
    }

    let cursor_size = Vec2::new(width / 8.0, width / 8.0);
    let cursor_position = Vec2::new(0.0, 250.0);
    let enc = commands.spawn((
        Sprite::from_color(Color::srgb(0.25, 0.75, 0.25), cursor_size),
        Transform::from_translation(cursor_position.extend(-0.1)),
        Cursor(0),
        Portrait,
        DatingObj,
    ));

    let size = width / 9.0;

    commands.spawn((
        Sprite {
            custom_size: Some(Vec2::new(size, size)),
            image: asset_server.load("Icons/ExitShip_Icon.png"),
            ..Default::default()
        },
        Transform::from_translation(Vec2::new(0.0, -height / 3.0).extend(0.0)),
        Portrait,
        DatingObj,
    ));

    for (idx, i) in context.all_characters.iter().enumerate() {
        let portrait = get_portrait(
            i.character,
            Vec2::new(size, size),
            &asset_server,
            context.flags.clone(),
        );
        let box_position = Vec2::new((idx as f32 * size * 1.2) - width / 2.5, 250.0);
        // if let Some(mission_var) = i.current_dialogue {
        //     let box_size = Vec2::new(size / 1.5, size / 1.5);
        //     let box_position = box_position + Vec2::new(0.0, -150.0);
        //     let enc = commands.spawn((
        //         Sprite::from_color(Color::srgb(0.75, 0.25, 0.25), box_size),
        //         Transform::from_translation(box_position.extend(0.0)),
        //         DatingObj,
        //         MissionNot,
        //     ));
        // };

        let box_size = Vec2::new(size, size);
        commands
            .spawn((
                Sprite::from_color(Color::srgb(0.75, 0.75, 0.75), box_size),
                Transform::from_translation(box_position.extend(0.0)),
                Portrait,
                DatingObj,
            ))
            .with_children(|builder| {
                builder.spawn((portrait, Transform::from_translation(Vec3::Z)));
            });

        // if let death_flag = context.flags.contains_key(i.character); //
        // if (death_flag < 0) {
        //     //They are dead
        //     let box_size = Vec2::new(size, size);
        //     commands
        //         .spawn((
        //             Sprite::from_color(Color::srgb(0.0, 0.0, 0.0), box_size * 0.9),
        //             Transform::from_translation(box_position.extend(3.0)),
        //             Portrait,
        //             DatingObj,
        //         ))
        //         .with_children(|builder| {
        //             builder.spawn((portrait, Transform::from_translation(Vec3::Z)));
        //         });
    }

    let text_justification = JustifyText::Center;
}

fn get_portrait(
    character: CharactersType,
    size: Vec2,
    asset_server: &Res<AssetServer>,
    flags: HashMap<String, isize>,
) -> Sprite {
    return match character {
        CharactersType::Joe if !flags.get("JoeDead").is_some_and(|x| *x == 1) => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Janitor Joe-Recovered.png"),
            ..Default::default()
        },
        CharactersType::Jule if !flags.get("JuleDead").is_some_and(|x| *x == 1) => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Character_General_Jule.png"),
            ..Default::default()
        },
        CharactersType::Fredrick if !flags.get("FredrickDead").is_some_and(|x| *x == 1) => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Character_Twin_Fredrick.png"),
            ..Default::default()
        },

        CharactersType::Diedrick if !flags.get("DiedrickDead").is_some_and(|x| *x == 1) => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Character_Twin_Dedrick.png"),
            ..Default::default()
        },

        CharactersType::Carle if !flags.get("CarleDead").is_some_and(|x| *x == 1) => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Character_Carly.png"),

            ..Default::default()
        },
        CharactersType::Liv if !flags.get("LivDead").is_some_and(|x| *x == 1) => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Character_Liv.png"),
            ..Default::default()
        },
        CharactersType::Cat if !flags.get("CatDead").is_some_and(|x| *x == 1) => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Character_cat.png"),
            ..Default::default()
        },
        CharactersType::You => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Character_Player.png"),
            ..Default::default()
        },
        _ => Sprite {
            custom_size: Some(size),
            image: asset_server.load("Portraits/Character_Liv.png"),
            color: Color::srgba(0.00, 0.00, 0.00, 0.8),
            ..default()
        },
    };
}

#[derive(Component)]
struct EmptyScene;

fn start_talking(
    mut commands: Commands,
    context: ResMut<DatingContext>,
    mut query: Query<&mut Transform, With<Cursor>>,
    asset_server: Res<AssetServer>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let window = windows.single();
    let width = window.resolution.width();
    let height = window.resolution.height();

    let font = asset_server.load("fonts/Pixelfont/slkscr.ttf");
    let text_font = TextFont {
        font: font.clone(),
        font_size: 50.0,
        ..default()
    };

    let slightly_smaller_text_font = TextFont {
        font,
        font_size: 27.0,
        ..default()
    };

    let talk_size = Vec2::new(width / 1.3, height / 3.0);
    let talk_position = Vec2::new(width / 8.0, -height / 2.7);

    let text = &context.selected_scene.text;

    let first = text.first().cloned().unwrap_or((None, String::new()));

    let dialogue = first.1.clone();
    let person = first.0;
    commands
        .spawn((
            Sprite {
                custom_size: Some(talk_size),
                image: asset_server.load("Textbox/Textbox.png"),
                ..Default::default()
            },
            Transform::from_translation(talk_position.extend(1.0)),
            TalkObj,
        ))
        .with_children(|builder| {
            builder
                .spawn((
                    TextColor(Color::srgb(0.0, 0.0, 0.0)),
                    Text2d::new(dialogue),
                    TextBox(0),
                    slightly_smaller_text_font.clone(),
                    TextLayout::new(JustifyText::Left, LineBreak::WordBoundary),
                    // Wrap text in the rectangle
                    TextBounds::from(talk_size * 0.75),
                    // ensure the text is drawn on top of the box
                    Transform::from_translation(Vec3::Z),
                ))
                .insert_if(EmptyScene, || text.is_empty());
        });

    //Who is talking
    if let Some(real_preson) = person {
        commands
            .spawn((
                Sprite {
                    custom_size: Some(talk_size),
                    image: asset_server.load("Textbox/Textbox-NameAddOn.png"),
                    ..Default::default()
                },
                Transform::from_translation(
                    (talk_position + Vec2::new(talk_size.x / 2.0 - width / 6.0, talk_size.y / 3.0))
                        .extend(0.8),
                ),
                TalkObj,
            ))
            .with_children(|builder| {
                builder.spawn((
                    Text2d::new(format!("{:?}", real_preson)),
                    TextColor(Color::srgb(0.0, 0.0, 0.0)),
                    NameBox,
                    slightly_smaller_text_font.clone(),
                    TextLayout::new(JustifyText::Left, LineBreak::WordBoundary),
                    // Wrap text in the rectangle
                    TextBounds::from(talk_size),
                    // ensure the text is drawn on top of the box
                    Transform::from_translation(Vec3::new(-85.0, 25.0, 1.0)),
                ));
            });

        //Look at sexy person talking
        commands.spawn((
            get_portrait(
                real_preson,
                Vec2::new(width / 2.0, width / 2.0),
                &asset_server,
                context.flags.clone(),
            ),
            Transform::from_translation(Vec2::new(-width / 4.0, -height / 10.0).extend(-0.5)),
            TalkObj,
            Portrait,
        ));
    }
}

fn on_choosing(
    mut commands: Commands,
    mut context: ResMut<DatingContext>,
    mut query: Query<&mut Transform, With<Cursor>>,
    asset_server: Res<AssetServer>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let window = windows.single();
    let width = window.resolution.width();
    let height = window.resolution.height();

    let font = asset_server.load("fonts/Pixelfont/slkscr.ttf");
    let text_font = TextFont {
        font: font.clone(),
        font_size: 50.0,
        ..default()
    };

    let slightly_smaller_text_font = TextFont {
        font,
        font_size: 27.0,
        ..default()
    };

    let option_size = Vec2::new(width / 2.0, height / 5.0);
    let option_position_1 = Vec2::new(0.0, height / 4.0);
    let option_position_2 = Vec2::new(0.0, -height / 4.0);
    for (label, id) in context
        .selected_scene
        .choice
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
    {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.20, 0.7, 0.20), option_size * 1.2),
            Transform::from_translation(option_position_1.extend(-0.5)),
            Cursor(0),
        ));
        commands
            .spawn((
                Sprite::from_color(Color::srgb(0.20, 0.3, 0.70), option_size),
                Transform::from_translation(option_position_1.extend(0.0)),
                ChoiceObj(id.clone()),
            ))
            .with_children(|builder| {
                builder.spawn((
                    Text2d::new(label),
                    slightly_smaller_text_font.clone(),
                    TextLayout::new(JustifyText::Left, LineBreak::WordBoundary),
                    TextBounds::from(option_size * 0.85),
                    Transform::from_translation(Vec3::Z),
                    TextColor(Color::srgb(0.0, 0.0, 0.0)),
                ));
            });
    }
    context.cursor = 0;
}

fn choose_move(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut ChoiceObj, With<ChoiceObj>>,
    mut cursor_query: Query<&mut Transform, With<Cursor>>,
    mut context: ResMut<DatingContext>,
    mut tmp: ResMut<NextState<DatingState>>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    // Consider changing font-size instead of scaling the transform. Scaling a Text2D will scale the
    // rendered quad, resulting in a pixellated look.

    let down = keyboard_input.just_pressed(KeyCode::KeyS)
        || keyboard_input.just_pressed(KeyCode::ArrowDown);
    let up =
        keyboard_input.just_pressed(KeyCode::KeyW) || keyboard_input.just_pressed(KeyCode::ArrowUp);
    let confirm = keyboard_input.just_pressed(KeyCode::Enter)
        || keyboard_input.just_pressed(KeyCode::Space)
        || keyboard_input.just_pressed(KeyCode::KeyZ);

    if up && context.cursor == 1 {
        context.cursor = 0;
    } else if down && context.cursor == 0 {
        context.cursor = 1;
    }

    let height = windows.single().resolution.height();

    if confirm {
        if let Some(choices) = context.selected_scene.choice.clone() {
            if (choices[context.cursor as usize].1).to_lowercase() == "return" {
                tmp.set(DatingState::Chilling);
            } else {
                for scene in context.scenes.clone() {
                    if scene.id == choices[context.cursor as usize].1 {
                        context.selected_scene = scene;
                        tmp.set(DatingState::Talking);
                        break;
                    };
                }
            }
        }
    }

    for mut transform in &mut cursor_query {
        transform.translation.y = match context.cursor {
            1 => -height / 4.0,
            _ => height / 4.0,
        };
    }
}

fn talking_action(
    time: Res<Time>,
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut new_scene: Local<bool>,
    query: Single<
        (Entity, &mut TextBox, &mut Text2d, Option<&EmptyScene>),
        (With<TextBox>, Without<Portrait>, Without<NameBox>),
    >,
    mut name_query: Query<Entity, (With<NameBox>, Without<TextBox>)>,
    mut face_query: Query<Entity, With<Portrait>>,
    mut context: ResMut<DatingContext>,
    asset_server: Res<AssetServer>,
    mut tmp: ResMut<NextState<DatingState>>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
    background: Single<&mut Sprite, With<Background>>,
) {
    let confirm = keyboard_input.just_pressed(KeyCode::Enter)
        || keyboard_input.just_pressed(KeyCode::Space)
        || keyboard_input.just_pressed(KeyCode::KeyZ);
    let escape = keyboard_input.just_pressed(KeyCode::Escape);

    let (entity, mut textbox, mut text, is_empty) = query.into_inner();

    let mut background = background.into_inner();

    if context.selected_scene.black {
        background.color = Color::srgba(0.0, 0.0, 0.0, 1.0);
    } else {
        background.color = Color::srgba(1.0, 1.0, 1.0, 1.0);
    }

    if escape {
        tmp.set(DatingState::Chilling);
    } else if confirm || *new_scene || is_empty.is_some() {
        if confirm {
            textbox.0 += 1;
        }
        if *new_scene {
            *new_scene = false;
        }

        if textbox.0 < context.selected_scene.text.len() {
            let dialogue = context.selected_scene.text[textbox.0].1.clone();

            *text = Text2d::new(dialogue);

            for entity in &mut name_query {
                commands.entity(entity).despawn_recursive();
            }
            for entity in &mut face_query {
                commands.entity(entity).despawn_recursive();
            }

            let font = asset_server.load("fonts/Pixelfont/slkscr.ttf");
            let text_font = TextFont {
                font: font.clone(),
                font_size: 50.0,
                ..default()
            };

            let slightly_smaller_text_font = TextFont {
                font,
                font_size: 27.0,
                ..default()
            };

            if let Some(new_person) = dbg!(context.selected_scene.text[(*textbox).0].0.clone()) {
                let window = windows.single();
                let width = window.resolution.width();
                let height = window.resolution.height();
                let talk_size = Vec2::new(width / 1.3, height / 3.0);
                let talk_position = Vec2::new(width / 8.0, -height / 2.7);
                commands
                    .spawn((
                        Sprite {
                            custom_size: Some(talk_size),
                            image: asset_server.load("Textbox/Textbox-NameAddOn.png"),
                            ..Default::default()
                        },
                        Transform::from_translation(
                            (talk_position
                                + Vec2::new(talk_size.x / 2.0 - width / 6.0, talk_size.y / 3.0))
                            .extend(0.8),
                        ),
                        TalkObj,
                    ))
                    .with_children(|builder| {
                        builder.spawn((
                            TextColor(Color::srgb(0.0, 0.0, 0.0)),
                            Text2d::new(format!("{:?}", new_person)),
                            NameBox,
                            slightly_smaller_text_font.clone(),
                            TextLayout::new(JustifyText::Left, LineBreak::WordBoundary),
                            // Wrap text in the rectangle
                            TextBounds::from(talk_size),
                            // ensure the text is drawn on top of the box
                            Transform::from_translation(Vec3::new(-85.0, 25.0, 1.0)),
                        ));
                    });

                //Look at sexy person talking
                commands.spawn((
                    get_portrait(
                        new_person,
                        Vec2::new(width / 2.0, width / 2.0),
                        &asset_server,
                        context.flags.clone(),
                    ),
                    Transform::from_translation(
                        Vec2::new(-width / 4.0, -height / 10.0).extend(-0.5),
                    ),
                    TalkObj,
                    Portrait,
                ));
            }
        } else {
            //We have finished reading

            // add the mission
            if let Some(mission) = context.selected_scene.mission {
                context.gathered_mission.push(mission);
            }

            // set outcomes
            if context.selected_scene.outcome.is_some() {
                println!("Added flag, but not implemented")
            }

            fn check_cond(
                flags: &mut HashMap<String, isize>,
                flag: Option<&String>,
                threshold: isize,
            ) -> bool {
                let Some(flag) = flag else {
                    return true;
                };

                let value = *flags.get(flag).unwrap_or(&0);

                if value >= threshold {
                    return true;
                }

                if threshold < 0 && value < threshold.abs() {
                    return true;
                }
                return false;
            }

            // if we have an option, start choosing
            if context.selected_scene.choice.is_some() {
                tmp.set(DatingState::Choosing);
            }
            // else, find more dialogue or quit
            else if context.selected_scene.next_scene.len() > 0 {
                for (cond, next_scene) in context.selected_scene.next_scene.clone() {
                    let mut passed = true;

                    for (ref flag, threshold) in cond {
                        passed = passed && check_cond(&mut context.flags, flag.as_ref(), threshold);
                    }

                    if passed {
                        if next_scene.to_lowercase() == "return" {
                            tmp.set(DatingState::Chilling);
                            break;
                        }

                        for scene in &context.scenes {
                            if scene.id == next_scene {
                                *new_scene = true;
                                context.selected_scene = scene.clone();
                                commands
                                    .entity(entity)
                                    .remove::<EmptyScene>()
                                    .insert_if(EmptyScene, || {
                                        context.selected_scene.text.is_empty()
                                    })
                                    .log_components();
                                textbox.0 = 0;
                                let dialogue = context.selected_scene.text[0].1.clone();
                                *text = Text2d::new(dialogue);
                                break;
                            };
                        }
                    }
                }
            } else {
                tmp.set(DatingState::Chilling);
            }
        }
    }
}

fn format_cond(cond: &Cond) {
    for (flag, threshold) in cond {
        if flag.is_none() {
            println!("(always true)");
        } else if *threshold > 0 {
            println!("{} >= {threshold}", flag.as_deref().unwrap_or("null"),)
        } else {
            println!(
                "{} < {}",
                flag.as_deref().unwrap_or("null"),
                threshold.abs()
            )
        }
    }
}

fn follow_mouse(
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut transform: Query<&mut Transform, With<FollowsMouse>>,
) {
    let Some(position) = q_windows.single().cursor_position() else {
        return;
    };

    for mut transform in &mut transform {
        transform.translation = position.extend(0.0);
    }
}
fn cursor_action(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Cursor>>,
    mut context: ResMut<DatingContext>,
    mut tmp: ResMut<NextState<DatingState>>,
    mut tmp_super: ResMut<NextState<GameState>>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    // Consider changing font-size instead of scaling the transform. Scaling a Text2D will scale the
    // rendered quad, resulting in a pixellated look.

    let left = keyboard_input.just_pressed(KeyCode::KeyA)
        || keyboard_input.just_pressed(KeyCode::ArrowLeft);
    let right = keyboard_input.just_pressed(KeyCode::KeyD)
        || keyboard_input.just_pressed(KeyCode::ArrowRight);
    let up =
        keyboard_input.just_pressed(KeyCode::KeyW) || keyboard_input.just_pressed(KeyCode::ArrowUp);
    let down = keyboard_input.just_pressed(KeyCode::KeyS)
        || keyboard_input.just_pressed(KeyCode::ArrowDown);
    let confirm = keyboard_input.just_pressed(KeyCode::Enter)
        || keyboard_input.just_pressed(KeyCode::Space)
        || keyboard_input.just_pressed(KeyCode::KeyZ);

    if confirm {
        if context.cursor == -5 {
            tmp.set(DatingState::Noting);
            tmp_super.set(GameState::Explore);
        } else {
            let talk_key = context.all_characters[(context.cursor + 3) as usize]
                .current_dialogue
                .clone();

            for scene in context.scenes.clone() {
                if scene.id == talk_key {
                    context.selected_scene = scene;
                    break;
                };
            }
            println!("set talking");
            tmp.set(DatingState::Talking);
        }
    }

    if right && context.cursor < 3 && context.cursor != -5 {
        context.cursor += 1
    } else if left && context.cursor > -3 {
        context.cursor -= 1
    } else if up && context.cursor == -5 {
        context.cursor = 0;
    } else if down && context.cursor != -5 {
        context.cursor = -5;
    }

    for mut transform in &mut query {
        let width = windows.single().resolution.width();
        let height = windows.single().resolution.height();
        if context.cursor != -5 {
            transform.translation.y = 250.0;
            transform.translation.x = (context.cursor as f32) * width / 7.5;
        } else {
            transform.translation.y = -height / 3.0;
            transform.translation.x = 0.0;
        }
    }
}
