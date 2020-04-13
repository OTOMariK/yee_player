// #![windows_subsystem = "windows"]

use std::{path::PathBuf, str::FromStr, sync::Arc};

pub mod entity;
use entity::{
    audio::{AudioBufferLoader, AudioController},
    button::{ButtonColors, ButtonResponse, ButtonState, StateButton},
    render::Transform,
    slider::{Slider, SliderColors},
    ButtonFn, ButtonFunctions, ControlledSliders, PlayingSpeed, Spawner, TargetValue,
};

pub mod buffer_player;
use buffer_player::SamplesBuffer;

pub mod renderer;
use renderer::{PiplineSetting, Renderer};

mod icon;
use icon::create_icon_data;

// MARK: consts
const FRAME_GAP: std::time::Duration = std::time::Duration::from_nanos(0_016_666_667);

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref FRAGMENT_SHADER_PATH: PathBuf =
        execute_or_relative_path("./asset/shader/frag.glsl.spv").unwrap();
    static ref VERTEX_SHADER_PATH: PathBuf =
        execute_or_relative_path("./asset/shader/vert.glsl.spv").unwrap();
    static ref SETTING_PATH: PathBuf =
        execute_or_relative_path("./asset/setting/setting.ron").unwrap();
    static ref PATH_OF_MUSIC_PATH: PathBuf =
        execute_or_relative_path("./asset/setting/music_path.txt").unwrap();
}

fn execute_or_relative_path(path: &str) -> Option<PathBuf> {
    if let Some(relative_path) = PathBuf::from_str(path).ok() {
        let exe_path = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(relative_path.clone());
        if exe_path.exists() {
            Some(exe_path)
        } else {
            Some(relative_path)
        }
    } else {
        None
    }
}

pub const NORMAL_BUTTON_COLOR: ButtonColors = ButtonColors {
    base_color: [0.0, 0.27, 0.5],
    hover_color: [0.6, 0.9, 1.0],
    press_color: [0.0, 0.1, 0.2],
};
pub const SLIDER_COLOR: ButtonColors = ButtonColors {
    base_color: [0.8, 0.5, 0.0],
    hover_color: [0.9, 0.8, 0.5],
    press_color: [0.7, 0.4, 0.0],
};
pub const LOOP_BUTTON_COLOR: ButtonColors = ButtonColors {
    base_color: [0.8, 0.5, 0.0],
    hover_color: [0.9, 0.8, 0.5],
    press_color: [0.7, 0.4, 0.0],
};
pub const LOADING_BUTTON_COLOR: ButtonColors = ButtonColors {
    base_color: [0.0, 0.05, 0.1],
    hover_color: [0.0, 0.04, 0.15],
    press_color: [0.0, 0.03, 0.05],
};

#[derive(Clone, Default, Debug)]
struct Input {
    mouse_location: Option<(f32, f32)>,
    mouse_pressing: bool,
    ctrl_pressing: bool,
    hover_file: bool,
    drop_file: Option<PathBuf>,
    exit: bool,
}

use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
struct Setting {
    window_width: f32,
    window_height: f32,
    max_speed: f32,
    min_speed: f32,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            window_width: 512.0,
            window_height: 512.0,
            max_speed: 2.0,
            min_speed: -2.0,
        }
    }
}

// MARK: main
fn main() -> Result<(), String> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Error)
        .filter_module("yee_player", log::LevelFilter::Info)
        .init();

    let setting: Setting = {
        let string = std::fs::read_to_string(&*SETTING_PATH).map_err(|e| {
            let err = format!("error opening {:?}: {:?}", &*SETTING_PATH, e);
            log::error!("{}", err);
            err
        })?;
        ron::de::from_str(string.as_str()).map_err(|e| {
            let err = format!("error parsing {:?}: {:?}", &*SETTING_PATH, e);
            log::error!("{}", err);
            err
        })?
    };

    let event_loop = winit::event_loop::EventLoop::new();

    let (window, size) = {
        let window = winit::window::WindowBuilder::new()
            .with_title("yee player")
            .with_inner_size(winit::dpi::LogicalSize {
                width: setting.window_width,
                height: setting.window_height,
            })
            .with_visible(false)
            .with_window_icon(Some(
                winit::window::Icon::from_rgba(create_icon_data(), 16, 16).unwrap(),
            ))
            .build(&event_loop)
            .map_err(|e| e.to_string())?;
        let center_position = {
            let window_size = window.outer_size();
            let srceen_size = window.current_monitor().size();
            winit::dpi::PhysicalPosition::new(
                (srceen_size.width - window_size.width) / 2,
                (srceen_size.height - window_size.height) / 2,
            )
        };
        window.set_outer_position(center_position);
        window.set_visible(true);

        let size = window.inner_size();
        (window, size)
    };

    let mut renderer = Renderer::init(&window, size)?;
    let render_pipeline = renderer.create_render_pipline(&PiplineSetting {
        vertex_shader_path: VERTEX_SHADER_PATH.clone(),
        fragment_shader_path: FRAGMENT_SHADER_PATH.clone(),
    })?;

    let audio_device = Arc::new(buffer_player::create_on_other_thread(|| -> _ {
        rodio::default_output_device().ok_or("can not get default audio output device!".to_string())
    })?);

    let (_universe, mut world, mut schedule) = {
        use legion::{
            entity::Entity,
            query::{IntoQuery, Read, Write},
            resource::{ResourceSet, Resources},
            schedule::Schedule,
            system::SystemBuilder,
            world::{Universe, World},
        };
        let universe = Universe::new();
        let mut world = universe.create_world();

        let mut resources = Resources::default();
        resources.insert(Input::default());
        let empty_buffer = Arc::new(SamplesBuffer::new(1, 48000, Vec::<i16>::new()));
        resources.insert(Arc::clone(&empty_buffer));
        let play_fn = Arc::new(|world: &mut World, _self_entity: Entity| {
            let query = Read::<AudioController<i16>>::query();
            let mut controller_entities = Vec::new();
            for (e, controller) in query.iter_entities_immutable(&world) {
                let speed = controller.get_speed();
                if speed == 0.0 {
                    let value = {
                        if let Some(v) = world.get_component::<PlayingSpeed>(e) {
                            v.0
                        } else {
                            1.0
                        }
                    };
                    controller.set_speed(value);
                } else {
                    controller.set_speed(0.0);
                    controller_entities.push((e, PlayingSpeed(speed)));
                }
            }
            for (e, v) in controller_entities {
                world.add_component(e, v);
            }
        }) as ButtonFn;

        let loop_fn = Arc::new(|world: &mut World, self_entity: Entity| {
            let query = Read::<AudioController<i16>>::query();
            for controller in query.iter_immutable(&world) {
                controller.set_loop_mode(true);
            }

            let unloop_fn = &Read::<ButtonFunctions>::fetch(&world.resources).unloop_fn;
            if let Some(mut self_fn) = world.get_component_mut::<ButtonFn>(self_entity) {
                *self_fn = Arc::clone(unloop_fn);
            }

            if let Some(mut colors) = world.get_component_mut::<ButtonColors>(self_entity) {
                *colors = LOOP_BUTTON_COLOR;
            }
        }) as ButtonFn;

        let unloop_fn = Arc::new(|world: &mut World, self_entity: Entity| {
            let query = Read::<AudioController<i16>>::query();
            for controller in query.iter_immutable(&world) {
                controller.set_loop_mode(false);
            }

            let loop_fn = &Read::<ButtonFunctions>::fetch(&world.resources).loop_fn;
            if let Some(mut self_fn) = world.get_component_mut::<ButtonFn>(self_entity) {
                *self_fn = Arc::clone(loop_fn);
            }

            if let Some(mut colors) = world.get_component_mut::<ButtonColors>(self_entity) {
                *colors = NORMAL_BUTTON_COLOR;
            }
        }) as ButtonFn;

        let load_fn = Arc::new(|world: &mut World, self_entity: Entity| {
            let file_path = std::fs::read_to_string(&*PATH_OF_MUSIC_PATH);
            match file_path {
                Err(e) => log::error!("error reading setting: {}", e),
                Ok(path) => {
                    function::load(world, self_entity, path);
                }
            }
        }) as ButtonFn;

        let stop_load_fn = Arc::new(|world: &mut World, self_entity: Entity| {
            let query = Read::<AudioBufferLoader<i16>>::query();
            for loader in query.iter_immutable(&world) {
                loader.stop_loading();
            }

            let load_fn = &Read::<ButtonFunctions>::fetch(&world.resources).load_fn;
            if let Some(mut self_fn) = world.get_component_mut::<ButtonFn>(self_entity) {
                *self_fn = Arc::clone(load_fn);
            }

            if let Some(mut colors) = world.get_component_mut::<ButtonColors>(self_entity) {
                *colors = NORMAL_BUTTON_COLOR;
            }
        }) as ButtonFn;

        resources.insert(ButtonFunctions {
            play_fn: Arc::clone(&play_fn),
            loop_fn: Arc::clone(&loop_fn),
            unloop_fn,
            load_fn: Arc::clone(&load_fn),
            stop_load_fn: Arc::clone(&stop_load_fn),
        });
        world.resources = resources;

        // MARK: entity
        world.insert(
            (),
            vec![
                (
                    StateButton::new(),
                    NORMAL_BUTTON_COLOR,
                    Transform {
                        location: [-1.0, 0.5],
                        size: [0.5, 0.5],
                        color: NORMAL_BUTTON_COLOR.base_color,
                    },
                    play_fn,
                ),
                (
                    StateButton::new(),
                    NORMAL_BUTTON_COLOR,
                    Transform {
                        location: [-0.5, 0.5],
                        size: [0.5, 0.5],
                        color: NORMAL_BUTTON_COLOR.base_color,
                    },
                    Arc::new(|world: &mut World, _self_entity: Entity| {
                        let query = Read::<AudioController<i16>>::query();
                        for controller in query.iter_immutable(&world) {
                            controller.set_speed(-controller.get_speed());
                        }
                    }),
                ),
                (
                    StateButton::new(),
                    NORMAL_BUTTON_COLOR,
                    Transform {
                        location: [0.0, 0.5],
                        size: [0.5, 0.5],
                        color: NORMAL_BUTTON_COLOR.base_color,
                    },
                    loop_fn,
                ),
            ],
        );
        let load_button_entity = world.insert(
            (),
            vec![(
                StateButton::new(),
                Slider::new(0.0, 0.0..1.0),
                TargetValue(0.0),
                LOADING_BUTTON_COLOR,
                SliderColors {
                    current_color: NORMAL_BUTTON_COLOR.base_color,
                    state_colors: NORMAL_BUTTON_COLOR,
                },
                Transform {
                    location: [0.5, 0.5],
                    size: [0.5, 0.5],
                    color: LOADING_BUTTON_COLOR.base_color,
                },
                stop_load_fn,
            )],
        )[0];

        {
            let args: Vec<String> = std::env::args().collect();
            if let Some(path) = args.get(1) {
                function::load(&mut world, load_button_entity, path.clone());
            } else {
                let file_path = std::fs::read_to_string(&*PATH_OF_MUSIC_PATH);
                match file_path {
                    Err(e) => log::error!("error reading setting: {}", e),
                    Ok(path) => {
                        function::load(&mut world, load_button_entity, path);
                    }
                }
            }
        }

        let slider_entities = world.insert(
            (),
            vec![
                (
                    StateButton::new(),
                    Slider::new(0.0, 0.0..1.0),
                    NORMAL_BUTTON_COLOR,
                    SliderColors {
                        current_color: SLIDER_COLOR.base_color,
                        state_colors: SLIDER_COLOR,
                    },
                    Transform {
                        location: [-1.0, 0.0],
                        size: [2.0, 0.5],
                        color: NORMAL_BUTTON_COLOR.base_color,
                    },
                ),
                (
                    StateButton::new(),
                    Slider::new(1.0, setting.min_speed..setting.max_speed),
                    NORMAL_BUTTON_COLOR,
                    SliderColors {
                        current_color: SLIDER_COLOR.base_color,
                        state_colors: SLIDER_COLOR,
                    },
                    Transform {
                        location: [-1.0, -0.5],
                        size: [2.0, 0.5],
                        color: NORMAL_BUTTON_COLOR.base_color,
                    },
                ),
                (
                    StateButton::new(),
                    Slider::new(0.25, 0.0..1.0),
                    NORMAL_BUTTON_COLOR,
                    SliderColors {
                        current_color: SLIDER_COLOR.base_color,
                        state_colors: SLIDER_COLOR,
                    },
                    Transform {
                        location: [-1.0, -1.0],
                        size: [2.0, 0.5],
                        color: NORMAL_BUTTON_COLOR.base_color,
                    },
                ),
            ],
        );
        {
            let controlled_sliders = ControlledSliders {
                time_slider: slider_entities[0],
                speed_slider: slider_entities[1],
                volume_slider: slider_entities[2],
            };
            world.insert(
                (),
                vec![(
                    AudioController::new_with_buffer(audio_device, empty_buffer),
                    controlled_sliders,
                )],
            );
        }
        // MARK: systems
        let update_button_and_slider_color = SystemBuilder::new("update_button_and_slider_color")
            .with_query(<(Write<SliderColors>, Read<StateButton>)>::query())
            .with_query(<(Read<StateButton>, Read<ButtonColors>, Write<Transform>)>::query())
            .build(|_, world, _, (slider_query, button_query)| {
                for (button, colors, mut transform) in button_query.iter(world) {
                    match button.get_state() {
                        ButtonState::Unhover => {
                            let speed = 0.17;
                            let r = smooth_to(transform.color[0], colors.base_color[0], speed);
                            let g = smooth_to(transform.color[1], colors.base_color[1], speed);
                            let b = smooth_to(transform.color[2], colors.base_color[2], speed);
                            transform.color = [r, g, b];
                        }
                        ButtonState::Hover => {
                            let speed = 0.2;
                            let r = smooth_to(transform.color[0], colors.hover_color[0], speed);
                            let g = smooth_to(transform.color[1], colors.hover_color[1], speed);
                            let b = smooth_to(transform.color[2], colors.hover_color[2], speed);
                            transform.color = [r, g, b];
                        }
                        ButtonState::Press => {
                            let speed = 0.6;
                            let r = smooth_to(transform.color[0], colors.press_color[0], speed);
                            let g = smooth_to(transform.color[1], colors.press_color[1], speed);
                            let b = smooth_to(transform.color[2], colors.press_color[2], speed);
                            transform.color = [r, g, b];
                        }
                    }
                }
                for (mut color, button) in slider_query.iter(world) {
                    match button.get_state() {
                        ButtonState::Unhover => {
                            let speed = 0.17;
                            let r = smooth_to(
                                color.current_color[0],
                                color.state_colors.base_color[0],
                                speed,
                            );
                            let g = smooth_to(
                                color.current_color[1],
                                color.state_colors.base_color[1],
                                speed,
                            );
                            let b = smooth_to(
                                color.current_color[2],
                                color.state_colors.base_color[2],
                                speed,
                            );
                            color.current_color = [r, g, b];
                        }
                        ButtonState::Hover => {
                            let speed = 0.2;
                            let r = smooth_to(
                                color.current_color[0],
                                color.state_colors.hover_color[0],
                                speed,
                            );
                            let g = smooth_to(
                                color.current_color[1],
                                color.state_colors.hover_color[1],
                                speed,
                            );
                            let b = smooth_to(
                                color.current_color[2],
                                color.state_colors.hover_color[2],
                                speed,
                            );
                            color.current_color = [r, g, b];
                        }
                        ButtonState::Press => {
                            let speed = 0.6;
                            let r = smooth_to(
                                color.current_color[0],
                                color.state_colors.press_color[0],
                                speed,
                            );
                            let g = smooth_to(
                                color.current_color[1],
                                color.state_colors.press_color[1],
                                speed,
                            );
                            let b = smooth_to(
                                color.current_color[2],
                                color.state_colors.press_color[2],
                                speed,
                            );
                            color.current_color = [r, g, b];
                        }
                    }
                }
            });

        let check_loader = SystemBuilder::new("check_loader")
            .write_component::<ButtonFn>()
            .write_component::<ButtonColors>()
            .write_component::<TargetValue>()
            .write_component::<Slider>()
            .read_resource::<ButtonFunctions>()
            .write_resource::<Arc<SamplesBuffer<i16>>>()
            .with_query(<(Write<AudioBufferLoader<i16>>, Read<Spawner>)>::query())
            .with_query(<(Write<AudioController<i16>>, Read<ControlledSliders>)>::query())
            .build(
                |commands, mut world, (funcs, audio_buffer), (query0, query1)| {
                    let mut audio_buffer_loaded = false;
                    for (entity, (mut buffer_loader, caller)) in query0.iter_entities(&mut world) {
                        if let Some(value) = buffer_loader.try_get_value() {
                            match value {
                                Err(e) => {
                                    log::error!("error loading audio: {}", e);

                                    if let Some(mut target_value) =
                                        world.get_component_mut::<TargetValue>(caller.0)
                                    {
                                        target_value.0 = 0.0;
                                    }
                                }
                                Ok(value) => {
                                    log::info!(
                                        "load success, audio length: {}s",
                                        value.get_duration().as_secs_f32()
                                    );
                                    **audio_buffer = Arc::new(value);
                                    audio_buffer_loaded = true;

                                    if let Some(mut target_value) =
                                        world.get_component_mut::<TargetValue>(caller.0)
                                    {
                                        target_value.0 = 1.0;
                                    }
                                }
                            }
                            if let Some(mut caller_fn) =
                                world.get_component_mut::<ButtonFn>(caller.0)
                            {
                                *caller_fn = Arc::clone(&funcs.load_fn);
                            }

                            if let Some(mut colors) =
                                world.get_component_mut::<ButtonColors>(caller.0)
                            {
                                *colors = NORMAL_BUTTON_COLOR;
                            }
                            commands.delete(entity);
                        } else if let Some(mut target_value) =
                            world.get_component_mut::<TargetValue>(caller.0)
                        {
                            target_value.0 = buffer_loader.get_progress();
                        }
                    }
                    if audio_buffer_loaded {
                        for (mut controller, sliders) in query1.iter(world) {
                            if !Arc::ptr_eq(controller.get_target_buffer(), audio_buffer) {
                                controller.set_target_buffer(Arc::clone(audio_buffer));
                                let buffer_duartion =
                                    controller.get_target_buffer().get_duration().as_secs_f32();

                                let mut time_slider = world
                                    .get_component_mut::<Slider>(sliders.time_slider)
                                    .unwrap();
                                time_slider.set_range(0.0..buffer_duartion);

                                if controller.get_speed() < 0.0 {
                                    controller.change_time(buffer_duartion);
                                } else {
                                    controller.change_time(0.0);
                                }
                            }
                        }
                    }
                },
            );

        let update_button = SystemBuilder::new("update_button")
            .read_resource::<Input>()
            .with_query(<(Write<StateButton>, Read<Transform>)>::query())
            .build(|_commands, world, input, query| {
                for (mut button, transform) in query.iter(world) {
                    let hover = if let Some(location) = &input.mouse_location {
                        is_in_box(&transform, location)
                    } else {
                        false
                    };
                    button.update_with_input(hover, input.mouse_pressing);
                }
            });

        let update_slider = SystemBuilder::new("update_slider")
            .read_resource::<Input>()
            .with_query(<(Write<Slider>, Read<StateButton>, Read<Transform>)>::query())
            .build(|_commands, mut world, resource, query| {
                let mouse_location = &resource.mouse_location;
                for (mut slider, button, transform) in query.iter(&mut world) {
                    match button.get_state() {
                        ButtonState::Unhover => {}
                        ButtonState::Hover => {}
                        ButtonState::Press => {
                            if let Some(location) = mouse_location {
                                let value = if transform.size[0].is_normal() {
                                    let (x, _) = relative_to_box(&transform, location);
                                    let v = slider.map_value_back(x / transform.size[0]);
                                    if resource.ctrl_pressing {
                                        (v * 20.0).round() / 20.0
                                    } else {
                                        v
                                    }
                                } else {
                                    slider.map_value(0.0)
                                };
                                slider.input_value(value);
                            }
                        }
                    }
                }
            });
        let update_slider_with_target_value = SystemBuilder::new("update_slider_with_target_value")
            .with_query(<(Write<Slider>, Read<TargetValue>)>::query())
            .build(|_, mut world, _, query| {
                for (mut slider, value) in query.iter(&mut world) {
                    let value = smooth_to(slider.get_value(), value.0, 0.4);
                    slider.set_value(value);
                }
            });

        let update_controller = SystemBuilder::new("update_controller")
            .read_component::<Slider>()
            .write_component::<Slider>()
            .with_query(<(Write<AudioController<i16>>, Read<ControlledSliders>)>::query())
            .build(|_, mut world, _resource, query| {
                for (controller, sliders) in query.iter(&mut world) {
                    {
                        let mut time_slider = world
                            .get_component_mut::<Slider>(sliders.time_slider)
                            .unwrap();
                        if let Some(v) = time_slider.take_input_value() {
                            controller.change_time(v);
                        }
                        time_slider.set_value(controller.get_time());
                    }
                    {
                        let mut speed_slider = world
                            .get_component_mut::<Slider>(sliders.speed_slider)
                            .unwrap();
                        if let Some(v) = speed_slider.take_input_value() {
                            controller.set_speed(v);
                        }
                        speed_slider.set_value(controller.get_speed());
                    }
                    {
                        let mut volume_slider = world
                            .get_component_mut::<Slider>(sliders.volume_slider)
                            .unwrap();
                        if let Some(v) = volume_slider.take_input_value() {
                            controller.set_volume(v);
                            volume_slider.set_value(controller.get_volume());
                        }
                    }
                }
            });

        let check_file_hover = SystemBuilder::new("check_file_hover")
            .write_component::<StateButton>()
            .write_component::<ButtonFn>()
            .write_component::<ButtonColors>()
            .write_component::<Slider>()
            .write_component::<TargetValue>()
            .write_resource::<Input>()
            .read_resource::<ButtonFunctions>()
            .with_query(<(Read<AudioBufferLoader<i16>>, Read<Spawner>)>::query())
            .build(move |command, world, (input, funcs), query| {
                if let None = query.iter_immutable(world).next() {
                    if input.hover_file {
                        if let Some(mut button) =
                            world.get_component_mut::<StateButton>(load_button_entity)
                        {
                            button.update_with_input(true, false);
                        }
                    }
                    if let Some(path) = input.drop_file.take() {
                        log::info!("loading {:?}", path);
                        command.insert(
                            (),
                            vec![(AudioBufferLoader::load(path), Spawner(load_button_entity))],
                        );

                        if let Some(mut self_fn) =
                            world.get_component_mut::<ButtonFn>(load_button_entity)
                        {
                            *self_fn = Arc::clone(&funcs.stop_load_fn);
                        }

                        if let Some(mut colors) =
                            world.get_component_mut::<ButtonColors>(load_button_entity)
                        {
                            *colors = LOADING_BUTTON_COLOR;
                        }

                        if let Some(mut slider) =
                            world.get_component_mut::<Slider>(load_button_entity)
                        {
                            slider.set_value(0.0);
                        }

                        if let Some(mut target_value) =
                            world.get_component_mut::<TargetValue>(load_button_entity)
                        {
                            target_value.0 = 0.0;
                        }
                    }
                } else {
                    input.drop_file.take();
                }
            });

        let execute_button = Box::new(|world: &mut World| {
            let query = <(Read<StateButton>, Read<ButtonFn>)>::query();
            let mut funcs_entities = Vec::new();
            // collect funcs to call
            for (entity, (button, func)) in query.iter_entities(world) {
                if let Some(response) = button.get_response() {
                    match response {
                        ButtonResponse::Hover => {}
                        ButtonResponse::Unhover => {}
                        ButtonResponse::Press => {}
                        ButtonResponse::Release => {
                            funcs_entities.push((Arc::clone(&func), entity));
                        }
                    }
                }
            }
            for (func, entity) in funcs_entities {
                func(world, entity);
            }
        });
        let schedule = Schedule::builder()
            .add_system(update_button_and_slider_color)
            .add_system(check_loader)
            .add_system(update_button)
            .add_system(update_slider)
            .add_system(update_slider_with_target_value)
            .add_system(update_controller)
            .add_system(check_file_hover)
            .flush()
            .add_thread_local_fn(execute_button)
            .build();
        (universe, world, schedule)
    };

    // MARK: run
    let mut next_time = std::time::Instant::now();
    let mut should_tick = false;
    event_loop.run(move |event, _, control_flow| {
        use winit::{
            event::{Event, KeyboardInput, StartCause, WindowEvent},
            event_loop::ControlFlow,
        };
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let now = std::time::Instant::now();
                should_tick = true;
                next_time += FRAME_GAP;
                while now > next_time + FRAME_GAP {
                    next_time += FRAME_GAP;
                }
            }
            // MouseInput
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CursorMoved { position, .. },
            } if window_id == window.id() => {
                let inner_size = window.inner_size();
                let x = (position.x as f32 / inner_size.width as f32) * 2.0 - 1.0;
                let y = 1.0 - (position.y as f32 / inner_size.height as f32) * 2.0;
                use legion::{query::Write, resource::ResourceSet};
                Write::<Input>::fetch(&world.resources).mouse_location = Some((x, y));
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CursorLeft { .. },
            } if window_id == window.id() => {
                use legion::{query::Write, resource::ResourceSet};
                Write::<Input>::fetch(&world.resources).mouse_location = None;
            }
            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::MouseInput {
                        state,
                        button: winit::event::MouseButton::Left,
                        ..
                    },
            } if window_id == window.id() => {
                use legion::{query::Write, resource::ResourceSet};
                let mouse_pressing = &mut Write::<Input>::fetch(&world.resources).mouse_pressing;
                match state {
                    winit::event::ElementState::Pressed => *mouse_pressing = true,
                    winit::event::ElementState::Released => *mouse_pressing = false,
                }
            }
            // KeyboardInput
            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
            } if window_id == window.id() => {
                use winit::event::VirtualKeyCode;
                match keycode {
                    VirtualKeyCode::Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    VirtualKeyCode::LControl | VirtualKeyCode::RControl => {
                        use legion::{query::Write, resource::ResourceSet};
                        let ctrl_pressing =
                            &mut Write::<Input>::fetch(&world.resources).ctrl_pressing;
                        match state {
                            winit::event::ElementState::Pressed => *ctrl_pressing = true,
                            winit::event::ElementState::Released => *ctrl_pressing = false,
                        }
                    }
                    _ => {}
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::HoveredFile(_),
            } if window_id == window.id() => {
                use legion::{query::Write, resource::ResourceSet};
                Write::<Input>::fetch(&world.resources).hover_file = true;
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::HoveredFileCancelled,
            } if window_id == window.id() => {
                use legion::{query::Write, resource::ResourceSet};
                Write::<Input>::fetch(&world.resources).hover_file = false;
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::DroppedFile(path),
            } if window_id == window.id() => {
                use legion::{query::Write, resource::ResourceSet};
                let mut input = Write::<Input>::fetch(&world.resources);
                input.drop_file = Some(path);
                input.hover_file = false;
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if window_id == window.id() => {
                renderer.resize(size);
            }
            Event::MainEventsCleared if should_tick => {
                schedule.execute(&mut world);
                use legion::{query::Read, resource::ResourceSet};
                if Read::<Input>::fetch(&world.resources).exit {
                    *control_flow = ControlFlow::Exit;
                } else {
                    window.request_redraw();
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let mut transforms = Vec::new();
                {
                    use legion::query::{IntoQuery, Read};
                    for (_, transform) in
                        <(Read<StateButton>, Read<Transform>)>::query().iter_immutable(&world)
                    {
                        transforms.push(*transform);
                    }
                    for (slider, slider_colors, transform) in
                        <(Read<Slider>, Read<SliderColors>, Read<Transform>)>::query()
                            .iter_immutable(&world)
                    {
                        let progress = Transform {
                            location: [transform.location[0], transform.location[1]],
                            size: [
                                transform.size[0] * slider.get_value_mapped(),
                                transform.size[1],
                            ],
                            color: slider_colors.current_color,
                        };
                        transforms.push(progress);
                    }
                }
                if let Err(e) = renderer.render(&transforms, &render_pipeline) {
                    log::error!("render error: {}", e);
                }
            }
            Event::RedrawEventsCleared => {
                *control_flow = ControlFlow::WaitUntil(next_time);
                if should_tick {
                    should_tick = false;
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::LoopDestroyed => {
                log::info!("exit");
            }
            _ => {}
        }
    });
}

fn is_in_box(transform: &Transform, position: &(f32, f32)) -> bool {
    let left = transform.location[0];
    let right = transform.location[0] + transform.size[0];
    let bottom = transform.location[1];
    let top = transform.location[1] + transform.size[1];
    position.0 > left && position.0 < right && position.1 > bottom && position.1 < top
}

fn relative_to_box(transform: &Transform, position: &(f32, f32)) -> (f32, f32) {
    let left = transform.location[0];
    let top = transform.location[1];
    (position.0 - left, position.1 - top)
}

fn smooth_to(current_value: f32, target_value: f32, change_speed: f32) -> f32 {
    current_value + (target_value - current_value) * change_speed
}

mod function {
    use super::LOADING_BUTTON_COLOR;
    use legion::{entity::Entity, query::Read, resource::ResourceSet, world::World};
    use std::{
        fmt::Debug,
        marker::{Send, Sync},
    };

    use super::entity::{
        audio::AudioBufferLoader, button::ButtonColors, slider::Slider, ButtonFn, ButtonFunctions,
        Spawner, TargetValue,
    };
    pub fn load<P: AsRef<std::path::Path> + Debug + Sync + Send + 'static>(
        world: &mut World,
        self_entity: Entity,
        file_path: P,
    ) {
        log::info!("loading {:?}", file_path);
        world.insert(
            (),
            vec![(AudioBufferLoader::load(file_path), Spawner(self_entity))],
        );

        let stop_load_fn = &Read::<ButtonFunctions>::fetch(&world.resources).stop_load_fn;
        if let Some(mut self_fn) = world.get_component_mut::<ButtonFn>(self_entity) {
            *self_fn = std::sync::Arc::clone(stop_load_fn);
        }

        if let Some(mut colors) = world.get_component_mut::<ButtonColors>(self_entity) {
            *colors = LOADING_BUTTON_COLOR;
        }

        if let Some(mut slider) = world.get_component_mut::<Slider>(self_entity) {
            slider.set_value(0.0);
        }

        if let Some(mut target_value) = world.get_component_mut::<TargetValue>(self_entity) {
            target_value.0 = 0.0;
        }
    }
}
