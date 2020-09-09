pub mod button;

pub mod slider;



pub mod render {
    pub use crate::renderer::Transform;
}

use legion::{Entity, Resources, World};
use std::sync::Arc;
pub type ButtonFn = Arc<dyn Fn(&mut World, &mut Resources, Entity) + Send + Sync + 'static>;

pub struct TargetValue(pub f32);

pub mod resource;
