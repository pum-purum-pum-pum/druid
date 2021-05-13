// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Implementation of features at the application scope.

use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::application::AppHandler;
use crate::scale::Scale;

use super::clipboard::Clipboard;
use super::window::Window;

use glutin::dpi::PhysicalPosition;

use glutin::dpi::LogicalSize;

#[cfg(windows)]
use glutin::platform::windows::WindowBuilderExtWindows;

use glutin::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder, GlRequest,
};
use skia_safe::{
    gpu::{gl::FramebufferInfo, BackendRenderTarget, SurfaceOrigin},
    ColorType, Surface,
};

use anyhow::{anyhow, Error};

const TARGET_FPS: u64 = 60;
/// Whether to render incrementaly in separate texture and then render it to screen
//#[cfg(not(target_os = "macos"))]
//const BLIT_CANVAS: bool = false;
//#[cfg(target_os = "macos")]
//const BLIT_CANVAS: bool = true;

type WindowedContext = glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>;

#[derive(Clone)]
pub(crate) struct Application {
    /// The mutable `Application` state.
    state: Rc<RefCell<State>>,
}

/// The mutable `Application` state.
struct State {
    /// Whether `Application::quit` has already been called.
    _quitting: bool,
    /// A collection of all the `Application` windows.
    window: Option<Rc<Window>>, // we only want to support one window for now
}

impl Application {
    pub fn new() -> Result<Application, Error> {
        #[cfg(not(target_os = "macos"))]
        {
            // using functions from druid here to supress warnings without changing druid's code (and hence being upstream)
            use super::super::shared::hardware_keycode_to_code;
            hardware_keycode_to_code(0);
        }
        //use super::super::strip_access_key;
        let state = Rc::new(RefCell::new(State {
            _quitting: false,
            window: None,
        }));
        Ok(Application { state })
    }

    pub fn add_window(&self, window: Rc<Window>) -> Result<(), Error> {
        borrow_mut!(self.state)?.window = Some(window);
        Ok(())
    }

    pub fn window(&self) -> Result<Rc<Window>, Error> {
        let state = borrow_mut!(self.state)?;
        state
            .window
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("No window"))
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        if let Err(e) = self.run_inner() {
            log::error!("{}", e);
        }
    }

    pub fn run_inner(self) -> Result<(), Error> {
        let window_size = self.window().unwrap().size()?;
        let event_loop = EventLoop::new();
        let logical_window_size = LogicalSize::new(window_size.width, window_size.height);

        // Open a window.
        let window_builder = WindowBuilder::new()
            .with_title("Minimal example")
            .with_inner_size(logical_window_size);
        #[cfg(windows)]
        let window_builder = window_builder.with_drag_and_drop(false);

        // Create an OpenGL 3.x context for Pathfinder to use.
        let gl_context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (4, 6),
                opengles_version: (3, 1),
            })
            // seems like double buffering is not supported for wayland in glutin
            //.with_double_buffer(Some(true))
            .build_windowed(window_builder, &event_loop)?;

        // Load OpenGL, and make the context current.
        let gl_context = unsafe { gl_context.make_current().map_err(|e| e.1)? };

        gl::load_with(|name| gl_context.get_proc_address(name));

        let mut gr_context = skia_safe::gpu::Context::new_gl(None, None)
            .ok_or_else(|| anyhow!("failed to create context"))?;

        let fb_info = {
            let mut fboid: gl::types::GLint = 0;
            unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

            FramebufferInfo {
                fboid: fboid.try_into()?,
                format: skia_safe::gpu::gl::Format::RGBA8.into(),
            }
        };

        fn create_surface(
            windowed_context: &WindowedContext,
            fb_info: FramebufferInfo,
            gr_context: &mut skia_safe::gpu::Context,
        ) -> Result<skia_safe::Surface, Error> {
            let pixel_format = windowed_context.get_pixel_format();
            let size = windowed_context.window().inner_size();
            let backend_render_target = BackendRenderTarget::new_gl(
                (size.width.try_into()?, size.height.try_into()?),
                pixel_format.multisampling.and_then(|s| s.try_into().ok()),
                pixel_format.stencil_bits.try_into()?,
                fb_info,
            );
            Surface::from_backend_render_target(
                gr_context,
                &backend_render_target,
                SurfaceOrigin::BottomLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .ok_or_else(|| anyhow!("No window"))
        }

        let mut surface = create_surface(&gl_context, fb_info, &mut gr_context)?;
        // It's not working on wayland for some reason.
        let _sf = gl_context.window().scale_factor() as f32;
        let scale = if let Ok(window) = self.window() {
            window.state().unwrap().scale
        } else {
            Scale::default()
        };

        surface.canvas().scale((scale.x() as f32, scale.y() as f32));

        let mut cursor_position = PhysicalPosition::new(0., 0.);
        let mut last_ts = Instant::now();
        let mut time = Duration::default();
        let mut frames_cnt = 0;
        let mut redraw_timestamp = Instant::now();
        // scheduler chould wake up us later so we want to say to it to wake up us earlier
        // We can even calculate it dynamicly as some average :)
        let scheduler_lag = Duration::from_millis(1);
        let frame_time = Duration::from_secs_f64(1. / TARGET_FPS as f64) - scheduler_lag;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            let _size = gl_context.window().inner_size();
            let _size = (_size.width as f32, _size.height as f32);
            {
                let main_window = self.window().unwrap();
                main_window.run_idle();
                let now = Instant::now();
                main_window.run_timers(now);
            }
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { input, .. },
                    ..
                } => {
                    let main_window = self.window().unwrap();
                    main_window.handle_key_press(input);
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(physical_size),
                    ..
                } => {
                    gl_context.resize(physical_size);
                    // TODO something with these unwraps
                    surface = create_surface(&gl_context, fb_info, &mut gr_context).unwrap();
                    surface.canvas().scale((scale.x() as f32, scale.y() as f32));
                    let main_window = self.window().unwrap();
                    main_window.screen_size_changed(physical_size).unwrap();
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    cursor_position = position;
                    let main_window = self.window().unwrap();
                    main_window.handle_motion_notify(position);
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseInput { button, state, .. },
                    ..
                } => {
                    let main_window = self.window().unwrap();
                    match state {
                        glutin::event::ElementState::Pressed => {
                            main_window.handle_button_press(cursor_position, button);
                        }
                        glutin::event::ElementState::Released => {
                            main_window.handle_button_release(cursor_position, button);
                        }
                    }
                }
                Event::RedrawRequested(_) => {
                    {
                        // frame rate
                        frames_cnt += 1;
                        let duration = Instant::now() - last_ts;
                        time += duration;
                        last_ts = Instant::now();
                        if time > Duration::from_secs(1) {
                            log::info!("{}", frames_cnt);
                            frames_cnt = 0;
                            time = time.max(Duration::from_secs(1)) - Duration::from_secs(1);
                        }
                    }
                    let surface_canvas = surface.canvas();
                    let main_window = self.window().unwrap();
                    main_window.render(&mut *surface_canvas).unwrap();
                    surface_canvas.flush();
                    gl_context.swap_buffers().unwrap();
                    redraw_timestamp = Instant::now();
                }
                _ => {
                    let since_last_redraw = Instant::now().duration_since(redraw_timestamp);
                    if since_last_redraw > frame_time {
                        gl_context.window().request_redraw();
                    } else {
                        let wait_time = frame_time - since_last_redraw;
                        //*control_flow = ControlFlow::WaitUntil(Instant::now() + wait_time);
                        // wayland..
                        std::thread::sleep(wait_time);
                    }
                }
            }
        });
    }

    pub fn quit(&self) {}

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    #[cfg(target_os = "macos")]
    pub fn hide(&self) {}

    #[cfg(target_os = "macos")]
    pub fn hide_others(&self) {}

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}
