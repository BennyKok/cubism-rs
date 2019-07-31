use serde::{Deserialize, Serialize};

use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Model3 {
    pub version: usize,
    #[serde(default, rename = "FileReferences")]
    pub file_references: FileReferences,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default, rename = "HitAreas")]
    pub hit_areas: Vec<HitArea>,
    pub layout: Option<Layout>,
}

impl Model3 {
    #[inline]
    pub fn from_str(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }

    #[inline]
    pub fn from_reader<R: std::io::Read>(r: R) -> serde_json::Result<Self> {
        serde_json::from_reader(r)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FileReferences {
    pub moc: Option<PathBuf>,
    #[serde(default)]
    pub textures: Vec<PathBuf>,
    pub pose: Option<PathBuf>,
    pub physics: Option<PathBuf>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub motions: Motions,
    #[serde(rename = "UserData")]
    pub user_data: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Group {
    pub target: String,
    pub name: String,
    pub ids: Vec<String>,
}

// TODO: Might very well be just a hashmap figure out whether these names should
// be hardcoded or not
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Motions {
    #[serde(default)]
    pub idle: Vec<Motion>,
    #[serde(default, rename = "TapBody")]
    pub tap_body: Vec<Motion>,
    #[serde(default, rename = "PinchIn")]
    pub pinch_in: Vec<Motion>,
    #[serde(default, rename = "PinchOut")]
    pub pinch_out: Vec<Motion>,
    #[serde(default)]
    pub shake: Vec<Motion>,
    #[serde(default, rename = "FlickHead")]
    pub flick_head: Vec<Motion>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Motion {
    pub file: PathBuf,
    #[serde(rename = "FadeInTime", default = "Motion::fade_time_default")]
    pub fade_in_time: f32,
    #[serde(rename = "FadeOutTime", default = "Motion::fade_time_default")]
    pub fade_out_time: f32,
}

impl Motion {
    fn fade_time_default() -> f32 {
        1.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Expression {
    pub name: String,
    pub file: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct HitArea {
    pub name: String,
    pub id: String,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Layout {
    #[serde(rename = "CenterX")]
    pub center_x: f32,
    #[serde(rename = "CenterY")]
    pub center_y: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[test]
fn json_samples_model3() {
    use std::iter::FromIterator;
    let path = std::path::PathBuf::from_iter(&[env!("CUBISM_CORE"), "Samples/Res"]);
    for model in &["Haru", "Hiyori", "Mark", "Natori"] {
        serde_json::from_str::<Model3>(
            &std::fs::read_to_string(&path.join([model, "/", model, ".model3.json"].concat()))
                .unwrap(),
        )
        .unwrap();
    }
}
