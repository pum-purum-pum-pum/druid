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

//! Web implementation of features at the application scope.

use std::rc::Rc;
use std::cell::RefCell;

use crate::application::AppHandler;

use super::clipboard::Clipboard;
use super::window::Window;
use anyhow::{anyhow, Context, Error};

const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const FFT_BUFSIZE: usize = 44100 / 8;
const EQ_NODES: usize = 200;

use std::{collections::VecDeque, convert::TryInto, fs, ops, sync::mpsc, time};
use glutin::dpi::LogicalSize;
#[cfg(windows)]
use glutin::platform::windows::WindowBuilderExtWindows;
use glutin::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder, GlRequest,
};
use skia_safe::{
    gpu::{gl::FramebufferInfo, BackendRenderTarget, SurfaceOrigin},
    Color, ColorType, Paint, PaintStyle, Path, Surface,
};

use crate::piet::{Piet, PietText, RenderContext};

type WindowedContext = glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>;

#[derive(Clone)]
pub(crate) struct Application {  
    /// The mutable `Application` state.
    state: Rc<RefCell<State>>,
}


/// The mutable `Application` state.
struct State {
    /// Whether `Application::quit` has already been called.
    quitting: bool,
    /// A collection of all the `Application` windows.
    window: Option<Rc<Window>>, // we only want to support one window for now
}

impl Application {
    pub fn new() -> Result<Application, Error> {
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            window: None,
        }));
        Ok(Application{state})
    }

    pub fn add_window(&self, window: Rc<Window>) -> Result<(), Error> {
        borrow_mut!(self.state)?.window = Some(window);
        Ok(())
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        let event_loop = EventLoop::new();
        let logical_window_size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);

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
            .build_windowed(window_builder, &event_loop)
            .unwrap();

        // Load OpenGL, and make the context current.
        let gl_context = unsafe { gl_context.make_current().unwrap() };

        gl::load_with(|name| gl_context.get_proc_address(name));

        let mut gr_context = skia_safe::gpu::Context::new_gl(None, None).unwrap();

        let fb_info = {
            let mut fboid: gl::types::GLint = 0;
            unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

            FramebufferInfo {
                fboid: fboid.try_into().unwrap(),
                format: skia_safe::gpu::gl::Format::RGBA8.into(),
            }
        };

        fn create_surface(
            windowed_context: &WindowedContext,
            fb_info: &FramebufferInfo,
            gr_context: &mut skia_safe::gpu::Context,
        ) -> skia_safe::Surface {
            let pixel_format = windowed_context.get_pixel_format();
            let size = windowed_context.window().inner_size();
            let backend_render_target = BackendRenderTarget::new_gl(
                (
                    size.width.try_into().unwrap(),
                    size.height.try_into().unwrap(),
                ),
                pixel_format.multisampling.map(|s| s.try_into().unwrap()),
                pixel_format.stencil_bits.try_into().unwrap(),
                *fb_info,
            );
            Surface::from_backend_render_target(
                gr_context,
                &backend_render_target,
                SurfaceOrigin::BottomLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .unwrap()
        };

        let mut surface = create_surface(&gl_context, &fb_info, &mut gr_context);
        let sf = gl_context.window().scale_factor() as f32;
        surface.canvas().scale((sf, sf));
    

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            let size = gl_context.window().inner_size();
            let size = (size.width as f32, size.height as f32);

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                }
                | Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(physical_size),
                    ..
                } => {
                    gl_context.resize(physical_size);
                    surface = create_surface(&gl_context, &fb_info, &mut gr_context);
                }
                Event::RedrawRequested(_) => {
                    let canvas = surface.canvas();
                    let mut state = borrow_mut!(self.state).unwrap();
                    let main_window = state.window.as_mut().unwrap();
                    main_window.run_idle();
                    main_window.render(canvas).unwrap();
                    surface.canvas().flush();
                    gl_context.swap_buffers().unwrap();
                }
                Event::MainEventsCleared => {
                    gl_context.window().request_redraw();
                }
                _ => {}
            }
        });
    }

    pub fn quit(&self) {}

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}

