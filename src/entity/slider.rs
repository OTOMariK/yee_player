use std::ops::Range;
pub struct Slider {
    // 0.0 .. 1.0
    value: f32,
    // real value that was not mapped
    input_value: Option<f32>,
    // mapped value range
    value_range: Range<f32>,
}

impl Slider {
    pub fn new(value: f32, value_range: Range<f32>) -> Self {
        let value = map_value(value, &value_range).min(1.0).max(0.0);
        Self {
            value,
            input_value: None,
            value_range,
        }
    }
    pub fn take_input_value(&mut self) -> Option<f32> {
        self.input_value.take()
    }
    pub fn input_value(&mut self, value: f32) {
        self.input_value = Some(value.min(self.value_range.end).max(self.value_range.start));
    }
    pub fn get_value(&self) -> f32 {
        self.map_value_back(self.value)
    }
    pub fn get_value_mapped(&self) -> f32 {
        self.value
    }
    pub fn set_value(&mut self, value: f32) {
        self.value = self.map_value(value).min(1.0).max(0.0);
    }
    pub fn set_range(&mut self, value_range: Range<f32>) {
        self.value_range = value_range;
    }
    pub fn get_range(&self) -> &Range<f32> {
        &self.value_range
    }
    pub fn map_value_back(&self, value: f32) -> f32 {
        map_value_back(value, &self.value_range)
    }
    pub fn map_value(&self, value: f32) -> f32 {
        map_value(value, &self.value_range)
    }
}

// 0.0 .. 1.0 to range.end .. range.start
fn map_value_back(value: f32, range: &Range<f32>) -> f32 {
    range.start + (range.end - range.start) * value
}
// range.end .. range.start to 0.0 .. 1.0
fn map_value(value: f32, range: &Range<f32>) -> f32 {
    if (range.end - range.start).is_normal() {
        (value - range.start) / (range.end - range.start)
    } else {
        0.0
    }
}

use super::button::ButtonColors;
pub struct SliderColors {
    pub current_color: [f32; 3],
    pub state_colors: ButtonColors,
}
