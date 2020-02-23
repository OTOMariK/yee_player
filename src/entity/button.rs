#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    Unhover,
    Hover,
    Press,
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonResponse {
    Hover,
    Unhover,
    Press,
    Release,
}

pub struct StateButton {
    state: ButtonState,
    response: Option<ButtonResponse>,
}

impl StateButton {
    pub fn new() -> Self {
        StateButton {
            state: ButtonState::Unhover,
            response: None,
        }
    }
    pub fn update_with_input(&mut self, hovering: bool, pressing: bool) {
        use ButtonState::*;
        self.response = None;
        match self.state {
            Unhover => match (hovering, pressing) {
                (true, false) => {
                    self.response = Some(ButtonResponse::Hover);
                    self.state = Hover;
                }
                (_, _) => {}
            },
            Hover => match (hovering, pressing) {
                (false, _) => {
                    self.response = Some(ButtonResponse::Unhover);
                    self.state = Unhover;
                }
                (true, false) => {}
                (true, true) => {
                    self.response = Some(ButtonResponse::Press);
                    self.state = Press;
                }
            },
            Press => match (hovering, pressing) {
                (false, false) => {
                    self.response = Some(ButtonResponse::Unhover);
                    self.state = Unhover;
                }
                (true, false) => {
                    self.response = Some(ButtonResponse::Release);
                    self.state = Hover;
                }
                (_, true) => {}
            },
        }
    }
    pub fn get_state(&self) -> &ButtonState {
        &self.state
    }
    pub fn get_response(&self) -> Option<ButtonResponse> {
        self.response
    }
}

pub struct ButtonColors {
    pub base_color: [f32; 3],
    pub hover_color: [f32; 3],
    pub press_color: [f32; 3],
}
