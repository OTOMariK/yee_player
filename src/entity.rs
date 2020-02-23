pub mod button;

pub mod slider;

pub mod audio {
    pub use crate::buffer_player::{AudioBufferLoader, AudioController};
}

pub mod render {
    pub use crate::renderer::Transform;
}

use legion::{entity::Entity, world::World};
use std::sync::Arc;
pub type ButtonFn = Arc<dyn Fn(&mut World, Entity) + Send + Sync + 'static>;

pub struct ButtonFunctions {
    pub play_fn: ButtonFn,
    pub loop_fn: ButtonFn,
    pub unloop_fn: ButtonFn,
    pub load_fn: ButtonFn,
    pub stop_load_fn: ButtonFn,
}

pub struct PlayingSpeed(pub f32);

pub struct Spawner(pub Entity);

pub struct ControlledSliders {
    pub time_slider: Entity,
    pub speed_slider: Entity,
    pub volume_slider: Entity,
}

pub struct TargetValue(pub f32);