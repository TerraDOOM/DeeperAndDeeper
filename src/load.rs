use crate::dating_sim::DatingScene;
//use serde::Deserialize;

pub fn load_scenes() -> Vec<DatingScene> {
    let json_file_path = std::path::Path::new("assets/Scenes/GenericScenes.json");

    let paths = vec![
        std::path::Path::new("assets/Scenes/GenericScenes.json"),
        std::path::Path::new("assets/Scenes/DiedrickScenes.json"),
        std::path::Path::new("assets/Scenes/FredrickScenes.json"),
        std::path::Path::new("assets/Scenes/JoeScenes.json"),
        std::path::Path::new("assets/Scenes/JuleScenes.json"),
        std::path::Path::new("assets/Scenes/CarleScenes.json"),
        std::path::Path::new("assets/Scenes/LivScenes.json"),
    ];

    let mut scenes: Vec<DatingScene> = vec![];
    for path in paths {
        let file = std::fs::File::open(path).expect("failed to open file");

        let scenes_one_file: Vec<DatingScene> =
            serde_json::from_reader(file).expect("error while reading or parsing");
        scenes.append(&mut scenes_one_file.clone());
    }
    scenes
}
