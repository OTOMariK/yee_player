use std::path::PathBuf;

#[derive(Clone, Default, Debug)]
pub struct Input {
    pub mouse_location: Option<(f32, f32)>,
    pub mouse_pressing: bool,
    pub ctrl_pressing: bool,
    pub hover_file: bool,
    pub drop_file: Option<PathBuf>,
    pub exit: bool,
}

pub struct SettingPath(pub PathBuf);

use serde::Deserialize;
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Setting {
    pub music_path: String,
    pub window_width: f32,
    pub window_height: f32,
    pub max_speed: f32,
    pub min_speed: f32,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            music_path: "./asset/music/example.ogg".to_string(),
            window_width: 512.0,
            window_height: 512.0,
            max_speed: 2.0,
            min_speed: -2.0,
        }
    }
}
impl Setting {
    pub fn load(path: &PathBuf) -> Result<Self, String> {
        let string = std::fs::read_to_string(path).map_err(|e| {
            let err = format!("error opening {:?}: {:?}", path, e);
            log::error!("{}", err);
            err
        })?;
        ron::de::from_str(string.as_str()).map_err(|e| {
            let err = format!("error parsing {:?}: {:?}", path, e);
            log::error!("{}", err);
            err
        })
    }
}

pub type MusicFileMetaData = Option<std::fs::Metadata>;

use super::ButtonFn;
pub struct ButtonFunctions {
    pub play_fn: ButtonFn,
    pub loop_fn: ButtonFn,
    pub unloop_fn: ButtonFn,
    pub load_fn: ButtonFn,
    pub stop_load_fn: ButtonFn,
}

pub mod audio {
    pub use crate::buffer_player::{AudioBufferLoader, AudioController};
    use legion::Entity;
    pub struct AudioLoader {
        pub loader: AudioBufferLoader<i16>,
        pub path: String,
        pub load_button_entity: Entity,
    }
    pub type AudioLoaderRes = Option<AudioLoader>;
}
pub struct PlayingSpeed(pub f32);

use legion::Entity;
pub struct ControlledSliders {
    pub time_slider: Entity,
    pub speed_slider: Entity,
    pub volume_slider: Entity,
}
