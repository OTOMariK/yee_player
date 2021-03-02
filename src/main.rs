#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, sync::Arc};

pub mod entity;
use entity::{
    button::{ButtonColors, ButtonResponse, ButtonState, StateButton},
    render::Transform,
    resource::{
        audio::{AudioController, AudioLoader, AudioLoaderRes},
        ButtonFunctions, ControlledSliders, Input, MusicFileMetaData, PlayingSpeed, Setting,
        SettingPath,
    },
    slider::{Slider, SliderColors},
    ButtonFn, TargetValue,
};

pub mod buffer_player;
use buffer_player::{AudioBufferLoader, SamplesBuffer};

pub mod renderer;
use renderer::{PiplineSetting, Renderer};

mod icon;
use icon::create_icon_data;

use legion::{
    query::{IntoQuery, Read, Write},
    world::EntityStore,
    Entity, Resources, Schedule, SystemBuilder, World,
};
// MARK: consts
const FRAME_GAP: std::time::Duration = std::time::Duration::from_nanos(0_016_666_667);

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

// MARK: main
fn main() -> Result<(), String> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Error)
        .filter_module("yee_player", log::LevelFilter::Trace)
        .init();

    // use another thread to create the OutputStream of rodio. avoid winit conflict.
    let (sender, receiver) = std::sync::mpsc::channel();
    let (sender_end, receiver_end) = std::sync::mpsc::channel();
    let _audio_stream_thread = std::thread::spawn(move || {
        let (_stream, stream_handle) = match rodio::OutputStream::try_default() {
            Err(e) => {
                let error_msg = format!("error getting default OutputStream: {:?}", e);
                sender.send(Err(error_msg)).unwrap();
                return;
            }
            Ok(output) => output,
        };
        sender.send(Ok(stream_handle)).unwrap();
        drop(sender);
        receiver_end.recv().unwrap();
        log::info!("audio_stream_thread end");
    });

    let stream_handle = receiver.recv().unwrap()?;
    drop(receiver);

    let shader_path: PathBuf = function::execute_or_relative_path("./asset/shader/shader.wgsl")?;
    let setting_path =
        SettingPath(function::execute_or_relative_path("./asset/setting/setting.ron").unwrap());
    let setting = Setting::load(&setting_path.0).unwrap_or_default();

    let event_loop = winit::event_loop::EventLoop::new();

    let (window, size) = {
        let winit_window_builder = winit::window::WindowBuilder::new();

        let window = winit_window_builder
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
            let srceen_size = {
                if let Some(monitor) = window.current_monitor() {
                    monitor.size()
                } else {
                    window_size
                }
            };
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

    let renderer = Renderer::init(&window, size)?;
    let render_pipeline = renderer.create_render_pipline(&PiplineSetting { shader_path })?;

    let (mut world, mut resources, mut schedule) = {
        let mut world = World::default();

        let play_fn: ButtonFn = Arc::new(
            |_world: &mut World, res: &mut Resources, _self_entity: Entity| {
                let controller = res.get::<AudioController<i16>>().unwrap();
                let speed = controller.get_speed();
                if speed == 0.0 {
                    let value = res.get::<PlayingSpeed>().unwrap().0;
                    controller.set_speed(value);
                } else {
                    res.get_mut::<PlayingSpeed>().unwrap().0 = speed;
                    controller.set_speed(0.0);
                }
            },
        );

        let loop_fn: ButtonFn = Arc::new(
            |world: &mut World, res: &mut Resources, self_entity: Entity| {
                let controller = res.get::<AudioController<i16>>().unwrap();
                controller.set_loop_mode(true);

                let unloop_fn: &ButtonFn = &res.get::<ButtonFunctions>().unwrap().unloop_fn;
                if let Some(mut entry) = world.entry(self_entity) {
                    if let Ok(self_fn) = entry.get_component_mut::<ButtonFn>() {
                        *self_fn = Arc::clone(unloop_fn);
                    }
                    if let Ok(colors) = entry.get_component_mut::<ButtonColors>() {
                        *colors = LOOP_BUTTON_COLOR;
                    }
                }
            },
        );

        let unloop_fn: ButtonFn = Arc::new(
            |world: &mut World, res: &mut Resources, self_entity: Entity| {
                let controller = res.get::<AudioController<i16>>().unwrap();
                controller.set_loop_mode(false);

                let loop_fn: &ButtonFn = &res.get::<ButtonFunctions>().unwrap().loop_fn;
                if let Some(mut entry) = world.entry(self_entity) {
                    if let Ok(self_fn) = entry.get_component_mut::<ButtonFn>() {
                        *self_fn = Arc::clone(loop_fn);
                    }
                    if let Ok(colors) = entry.get_component_mut::<ButtonColors>() {
                        *colors = NORMAL_BUTTON_COLOR;
                    }
                }
            },
        );

        let load_fn: ButtonFn = Arc::new(
            |world: &mut World, res: &mut Resources, self_entity: Entity| {
                let setting_path = res.get::<SettingPath>().unwrap();
                let mut setting = res.get_mut::<Setting>().unwrap();
                let new_setting = Setting::load(&setting_path.0).unwrap_or_default();
                if new_setting.window_width != setting.window_width
                    || new_setting.window_height != setting.window_height
                {
                    let window = res.get_mut::<winit::window::Window>().unwrap();
                    window.set_inner_size(winit::dpi::LogicalSize {
                        width: new_setting.window_width,
                        height: new_setting.window_height,
                    });
                    setting.window_width = new_setting.window_width;
                    setting.window_height = new_setting.window_height;
                }

                if new_setting.max_speed != setting.max_speed
                    || new_setting.min_speed != setting.min_speed
                {
                    let speed_slider_entity = res.get::<ControlledSliders>().unwrap().speed_slider;
                    if let Some(mut entry) = world.entry(speed_slider_entity) {
                        let speed_slider = entry.get_component_mut::<Slider>().unwrap();
                        speed_slider.set_range(new_setting.min_speed..new_setting.max_speed);
                    }
                    setting.max_speed = new_setting.max_speed;
                    setting.min_speed = new_setting.min_speed;
                }

                let new_music_path = function::execute_or_relative_path(&new_setting.music_path);
                match new_music_path {
                    Err(e) => log::error!("error getting music path: {}", e),
                    Ok(path) => match std::fs::metadata(&path) {
                        Err(e) => log::error!("error getting music metadata: {}", e),
                        Ok(meta) => {
                            let mut should_load = true;
                            if let Some(old_meta) = res.get::<MusicFileMetaData>().unwrap().as_ref()
                            {
                                if let (Ok(old_date), Ok(date)) =
                                    (old_meta.accessed(), meta.accessed())
                                {
                                    if old_date == date
                                        && setting.music_path == new_setting.music_path
                                    {
                                        should_load = false;
                                    }
                                }
                            }
                            if should_load {
                                function::load_music(
                                    world,
                                    res,
                                    self_entity,
                                    &new_setting.music_path,
                                );
                            }
                        }
                    },
                }
            },
        );

        let stop_load_fn: ButtonFn = Arc::new(
            |world: &mut World, res: &mut Resources, self_entity: Entity| {
                if let Some(loader) = res.get::<AudioLoaderRes>().unwrap().as_ref() {
                    loader.loader.stop_loading();
                }

                let load_fn: &ButtonFn = &res.get::<ButtonFunctions>().unwrap().load_fn;
                if let Some(mut entry) = world.entry(self_entity) {
                    if let Ok(self_fn) = entry.get_component_mut::<ButtonFn>() {
                        *self_fn = Arc::clone(load_fn);
                    }
                    if let Ok(colors) = entry.get_component_mut::<ButtonColors>() {
                        *colors = NORMAL_BUTTON_COLOR;
                    }
                }
            },
        ) as ButtonFn;

        // MARK: entity
        world.extend(vec![
            (
                StateButton::new(),
                NORMAL_BUTTON_COLOR,
                Transform {
                    location: [-1.0, 0.5],
                    size: [0.5, 0.5],
                    color: NORMAL_BUTTON_COLOR.base_color,
                },
                Arc::clone(&play_fn),
            ),
            (
                StateButton::new(),
                NORMAL_BUTTON_COLOR,
                Transform {
                    location: [-0.5, 0.5],
                    size: [0.5, 0.5],
                    color: NORMAL_BUTTON_COLOR.base_color,
                },
                Arc::new(
                    |_world: &mut World, res: &mut Resources, _self_entity: Entity| {
                        let controller = res.get::<AudioController<i16>>().unwrap();
                        controller.set_speed(-controller.get_speed());
                    },
                ),
            ),
            (
                StateButton::new(),
                NORMAL_BUTTON_COLOR,
                Transform {
                    location: [0.0, 0.5],
                    size: [0.5, 0.5],
                    color: NORMAL_BUTTON_COLOR.base_color,
                },
                Arc::clone(&loop_fn),
            ),
        ]);
        let load_button_entity = world.push((
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
            Arc::clone(&stop_load_fn),
        ));

        let slider_entities = world.extend(vec![
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
        ]);

        // Resources
        let mut resources = Resources::default();
        resources.insert(Input::default());
        let empty_buffer = Arc::new(SamplesBuffer::new(1, 48000, Vec::<i16>::new()));
        resources.insert(Arc::clone(&empty_buffer));
        resources.insert(window);
        resources.insert(renderer);
        resources.insert(ButtonFunctions {
            play_fn,
            loop_fn,
            unloop_fn,
            load_fn,
            stop_load_fn,
        });
        // setting
        resources.insert(setting_path);
        resources.insert(setting);
        resources.insert::<MusicFileMetaData>(None);
        // controller
        resources.insert(AudioController::new_with_buffer(
            &stream_handle,
            empty_buffer,
        ));
        let controlled_sliders = ControlledSliders {
            time_slider: slider_entities[0],
            speed_slider: slider_entities[1],
            volume_slider: slider_entities[2],
        };
        resources.insert(controlled_sliders);
        resources.insert(PlayingSpeed(1.0));
        // buffer loader
        resources.insert::<AudioLoaderRes>(None);

        {
            // command line support
            let args: Vec<String> = std::env::args().collect();
            if let Some(path) = args.get(1) {
                function::load_music(&mut world, &resources, load_button_entity, path);
            } else {
                let setting = resources.get::<Setting>().unwrap();
                function::load_music(
                    &mut world,
                    &resources,
                    load_button_entity,
                    &setting.music_path,
                );
            }
        }

        // MARK: systems
        let update_button_and_slider_color = SystemBuilder::new("update_button_and_slider_color")
            .with_query(<(Write<SliderColors>, Read<StateButton>)>::query())
            .with_query(<(Read<StateButton>, Read<ButtonColors>, Write<Transform>)>::query())
            .build(|_, world, _, (slider_query, button_query)| {
                button_query.for_each_mut(world, |(button, colors, mut transform)| {
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
                });

                slider_query.for_each_mut(world, |(mut color, button)| match button.get_state() {
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
                });
            });

        let check_loader = SystemBuilder::new("check_loader")
            .write_component::<ButtonFn>()
            .write_component::<ButtonColors>()
            .write_component::<TargetValue>()
            .write_component::<Slider>()
            .read_resource::<ButtonFunctions>()
            .write_resource::<Arc<SamplesBuffer<i16>>>()
            .write_resource::<AudioController<i16>>()
            .read_resource::<ControlledSliders>()
            .write_resource::<Setting>()
            .write_resource::<MusicFileMetaData>()
            .write_resource::<AudioLoaderRes>()
            .build(
                |_,
                 world,
                 (funcs, audio_buffer, controller, sliders, setting, meta_data, loader),
                 _| {
                    // query.for_each_mut(&mut query_world, |(entity, buffer_loader, caller)| {
                    let mut audio_buffer_loaded = false;
                    let mut drop_loader = false;
                    if let Some(loader) = loader.as_mut() {
                        if let Some(value) = loader.loader.try_get_value() {
                            drop_loader = true;
                            let value = match value {
                                Err(e) => {
                                    log::error!("error loading audio: {}", e);
                                    0.0
                                }
                                Ok(value) => {
                                    log::info!(
                                        "load success, audio length: {}s",
                                        value.get_duration().as_secs_f32()
                                    );
                                    **audio_buffer = Arc::new(value);
                                    audio_buffer_loaded = true;
                                    setting.music_path = loader.path.clone();
                                    **meta_data = std::fs::metadata(&setting.music_path).ok();
                                    1.0
                                }
                            };
                            // load_button visual stuff
                            if let Ok(mut entry) = world.entry_mut(loader.load_button_entity) {
                                if let Ok(target_value) = entry.get_component_mut::<TargetValue>() {
                                    target_value.0 = value;
                                }
                                if let Ok(colors) = entry.get_component_mut::<ButtonColors>() {
                                    *colors = NORMAL_BUTTON_COLOR;
                                }
                                if let Ok(caller_fn) = entry.get_component_mut::<ButtonFn>() {
                                    *caller_fn = Arc::clone(&funcs.load_fn);
                                }
                            }
                        } else if let Ok(mut entry) = world.entry_mut(loader.load_button_entity) {
                            if let Ok(target_value) = entry.get_component_mut::<TargetValue>() {
                                target_value.0 = loader.loader.get_progress();
                            }
                        }
                    }
                    if drop_loader {
                        **loader = None;
                    }
                    if audio_buffer_loaded {
                        if !Arc::ptr_eq(controller.get_target_buffer(), audio_buffer) {
                            controller.set_target_buffer(Arc::clone(audio_buffer));
                            let buffer_duartion =
                                controller.get_target_buffer().get_duration().as_secs_f32();

                            if let Ok(mut entry) = world.entry_mut(sliders.time_slider) {
                                if let Ok(time_slider) = entry.get_component_mut::<Slider>() {
                                    time_slider.set_range(0.0..buffer_duartion);
                                }
                            }

                            if controller.get_speed() < 0.0 {
                                controller.change_time(buffer_duartion);
                            } else {
                                controller.change_time(0.0);
                            }
                        }
                    }
                },
            );

        let update_button = SystemBuilder::new("update_button")
            .read_resource::<Input>()
            .with_query(<(Write<StateButton>, Read<Transform>)>::query())
            .build(|_commands, world, input, query| {
                query.for_each_mut(world, |(button, transform)| {
                    let hover = if let Some(location) = &input.mouse_location {
                        is_in_box(&transform, location)
                    } else {
                        false
                    };
                    button.update_with_input(hover, input.mouse_pressing);
                });
            });

        let update_slider = SystemBuilder::new("update_slider")
            .read_resource::<Input>()
            .with_query(<(Write<Slider>, Read<StateButton>, Read<Transform>)>::query())
            .build(|_commands, world, resource, query| {
                let mouse_location = &resource.mouse_location;
                query.for_each_mut(world, |(slider, button, transform)| {
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
                });
            });
        let update_slider_with_target_value = SystemBuilder::new("update_slider_with_target_value")
            .with_query(<(Write<Slider>, Read<TargetValue>)>::query())
            .build(|_, world, _, query| {
                for (slider, value) in query.iter_mut(world) {
                    let value = smooth_to(slider.get_value(), value.0, 0.4);
                    slider.set_value(value);
                }
            });

        let update_controller = SystemBuilder::new("update_controller")
            .read_component::<Slider>()
            .write_component::<Slider>()
            .write_resource::<AudioController<i16>>()
            .read_resource::<ControlledSliders>()
            .build(|_, world, (controller, sliders), _| {
                if let Ok(mut entry) = world.entry_mut(sliders.time_slider) {
                    if let Ok(time_slider) = entry.get_component_mut::<Slider>() {
                        if let Some(v) = time_slider.take_input_value() {
                            controller.change_time(v);
                        }
                        time_slider.set_value(controller.get_time());
                    }
                }

                if let Ok(mut entry) = world.entry_mut(sliders.speed_slider) {
                    if let Ok(speed_slider) = entry.get_component_mut::<Slider>() {
                        if let Some(v) = speed_slider.take_input_value() {
                            controller.set_speed(v);
                        }
                        speed_slider.set_value(controller.get_speed());
                    }
                }

                if let Ok(mut entry) = world.entry_mut(sliders.volume_slider) {
                    if let Ok(volume_slider) = entry.get_component_mut::<Slider>() {
                        if let Some(v) = volume_slider.take_input_value() {
                            controller.set_volume(v);
                        }
                        volume_slider.set_value(controller.get_volume());
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
            .write_resource::<AudioLoaderRes>()
            .build(move |_, world, (input, funcs, loader), _| {
                // when there is no AudioBufferLoader exist, load the dropped file
                if loader.is_none() {
                    if let Ok(mut entry) = world.entry_mut(load_button_entity) {
                        // highlight the load_button when hovering file
                        if input.hover_file {
                            if let Ok(button) = entry.get_component_mut::<StateButton>() {
                                button.update_with_input(true, false);
                            }
                        }
                        if let Some(path) = input.drop_file.take() {
                            match path.into_os_string().into_string() {
                                Err(e) => {
                                    log::error!("dropped file has invalid path: {:?}", e);
                                }
                                Ok(path) => {
                                    log::info!("loading {:?}", path);
                                    **loader = Some(AudioLoader {
                                        loader: AudioBufferLoader::load(path.clone()),
                                        path: path.clone(),
                                        load_button_entity,
                                    });

                                    if let Ok(self_fn) = entry.get_component_mut::<ButtonFn>() {
                                        *self_fn = Arc::clone(&funcs.stop_load_fn);
                                    }

                                    if let Ok(colors) = entry.get_component_mut::<ButtonColors>() {
                                        *colors = LOADING_BUTTON_COLOR;
                                    }

                                    if let Ok(slider) = entry.get_component_mut::<Slider>() {
                                        slider.set_value(0.0);
                                    }

                                    if let Ok(mut target_value) =
                                        entry.get_component_mut::<TargetValue>()
                                    {
                                        target_value.0 = 0.0;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    input.drop_file.take();
                }
            });

        let execute_button = Box::new(|world: &mut World, res: &mut Resources| {
            let mut query = <(Entity, Read<StateButton>, Read<ButtonFn>)>::query();
            let mut funcs_entities = Vec::new();
            // collect funcs to call
            for (entity, button, func) in query.iter(world) {
                if let Some(response) = button.get_response() {
                    match response {
                        ButtonResponse::Hover => {}
                        ButtonResponse::Unhover => {}
                        ButtonResponse::Press => {}
                        ButtonResponse::Release => {
                            funcs_entities.push((Arc::clone(&func), *entity));
                        }
                    }
                }
            }
            for (func, entity) in funcs_entities {
                func(world, res, entity);
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
        (world, resources, schedule)
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
            } if window_id == resources.get::<winit::window::Window>().unwrap().id() => {
                let inner_size = resources
                    .get::<winit::window::Window>()
                    .unwrap()
                    .inner_size();
                let x = (position.x as f32 / inner_size.width as f32) * 2.0 - 1.0;
                let y = 1.0 - (position.y as f32 / inner_size.height as f32) * 2.0;
                resources.get_mut::<Input>().unwrap().mouse_location = Some((x, y));
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CursorLeft { .. },
            } if window_id == resources.get::<winit::window::Window>().unwrap().id() => {
                resources.get_mut::<Input>().unwrap().mouse_location = None;
            }
            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::MouseInput {
                        state,
                        button: winit::event::MouseButton::Left,
                        ..
                    },
            } if window_id == resources.get::<winit::window::Window>().unwrap().id() => {
                let mouse_pressing = &mut resources.get_mut::<Input>().unwrap().mouse_pressing;
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
            } if window_id == resources.get::<winit::window::Window>().unwrap().id() => {
                use winit::event::VirtualKeyCode;
                match keycode {
                    VirtualKeyCode::Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    VirtualKeyCode::LControl | VirtualKeyCode::RControl => {
                        let ctrl_pressing =
                            &mut resources.get_mut::<Input>().unwrap().ctrl_pressing;
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
            } if window_id == resources.get::<winit::window::Window>().unwrap().id() => {
                resources.get_mut::<Input>().unwrap().hover_file = true;
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::HoveredFileCancelled,
            } if window_id == resources.get::<winit::window::Window>().unwrap().id() => {
                resources.get_mut::<Input>().unwrap().hover_file = false;
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::DroppedFile(path),
            } if window_id == resources.get::<winit::window::Window>().unwrap().id() => {
                let mut input = resources.get_mut::<Input>().unwrap();
                input.drop_file = Some(path);
                input.hover_file = false;
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if window_id == resources.get::<winit::window::Window>().unwrap().id() => {
                if size.width & size.height != 0 {
                    resources.get_mut::<Renderer>().unwrap().resize(size);
                }
            }
            Event::MainEventsCleared if should_tick => {
                schedule.execute(&mut world, &mut resources);
                if resources.get::<Input>().unwrap().exit {
                    *control_flow = ControlFlow::Exit;
                } else {
                    resources
                        .get::<winit::window::Window>()
                        .unwrap()
                        .request_redraw();
                }
            }
            Event::RedrawRequested(window_id)
                if window_id == resources.get::<winit::window::Window>().unwrap().id() =>
            {
                let inner_size = resources
                    .get::<winit::window::Window>()
                    .unwrap()
                    .inner_size();
                if inner_size.width & inner_size.height != 0 {
                    let mut transforms = Vec::new();
                    {
                        for (_, transform) in
                            <(Read<StateButton>, Read<Transform>)>::query().iter(&world)
                        {
                            transforms.push(*transform);
                        }
                        for (slider, slider_colors, transform) in
                            <(Read<Slider>, Read<SliderColors>, Read<Transform>)>::query()
                                .iter(&world)
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
                    if let Err(e) = resources
                        .get_mut::<Renderer>()
                        .unwrap()
                        .render(&transforms, &render_pipeline)
                    {
                        log::error!("render error: {}", e);
                    }
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
                sender_end.send(()).unwrap();
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
    use legion::{Entity, Resources, World};
    use std::{path::PathBuf, str::FromStr};

    use super::entity::{
        button::ButtonColors,
        resource::{audio::AudioBufferLoader, ButtonFunctions},
        slider::Slider,
        ButtonFn, TargetValue,
    };
    use crate::entity::resource::audio::{AudioLoader, AudioLoaderRes};

    pub fn execute_or_relative_path(path: &str) -> Result<PathBuf, String> {
        if let Ok(relative_path) = PathBuf::from_str(path) {
            if relative_path.is_absolute() {
                return Ok(relative_path);
            }
            if let Ok(exe) = std::env::current_exe() {
                if let Some(exe_path) = exe.parent() {
                    let exe_path = exe_path.join(relative_path.clone());
                    if exe_path.exists() {
                        return Ok(exe_path);
                    }
                }
            }
            Ok(relative_path)
        } else {
            Err("not a valid path".to_string())
        }
    }
    pub fn load_music(
        world: &mut World,
        res: &Resources,
        load_button_entity: Entity,
        path: &String,
    ) {
        let path_buf = execute_or_relative_path(path);
        match path_buf {
            Err(e) => {
                log::error!("error on getting path {}", e);
            }
            Ok(path_buf) => {
                log::info!("loading {:?}", path_buf);
                let mut loader = res.get_mut::<AudioLoaderRes>().unwrap();
                *loader = Some(AudioLoader {
                    loader: AudioBufferLoader::load(path_buf),
                    path: path.clone(),
                    load_button_entity,
                });
                let stop_load_fn = &res.get::<ButtonFunctions>().unwrap().stop_load_fn;
                if let Some(mut entry) = world.entry(load_button_entity) {
                    if let Ok(self_fn) = entry.get_component_mut::<ButtonFn>() {
                        *self_fn = std::sync::Arc::clone(stop_load_fn);
                    }

                    if let Ok(colors) = entry.get_component_mut::<ButtonColors>() {
                        *colors = LOADING_BUTTON_COLOR;
                    }

                    if let Ok(slider) = entry.get_component_mut::<Slider>() {
                        slider.set_value(0.0);
                    }

                    if let Ok(mut target_value) = entry.get_component_mut::<TargetValue>() {
                        target_value.0 = 0.0;
                    }
                }
            }
        }
    }
}
