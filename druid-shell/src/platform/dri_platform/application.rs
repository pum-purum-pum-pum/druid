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

use skia_safe::{
    gpu::{gl::FramebufferInfo, BackendRenderTarget, SurfaceOrigin},
    Color, ColorType, Surface
};

use anyhow::{anyhow, Error};

use dri::gl::*;
use dri::kms::{drm_screen_height, drm_screen_width, init, swap_buffers};

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
        unsafe { init() };
        let mut gr_context =
            skia_safe::gpu::Context::new_gl(None, None).expect("failed to create skia gl context");

        let fb_info = {
            let mut fboid: GLint = 0;
            unsafe { glGetIntegerv(GL_FRAMEBUFFER_BINDING, &mut fboid) };

            FramebufferInfo {
                fboid: fboid.try_into().unwrap(),
                format: skia_safe::gpu::gl::Format::RGBA8.into(),
            }
        };

        fn create_surface(
            fb_info: FramebufferInfo,
            gr_context: &mut skia_safe::gpu::Context,
        ) -> skia_safe::Surface {
            let backend_render_target = BackendRenderTarget::new_gl(
                (unsafe { drm_screen_width() }, unsafe {
                    drm_screen_height()
                }),
                None,
                0,
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
            .unwrap()
        }
        let mode_size = (unsafe { drm_screen_width() }, unsafe {
                    drm_screen_height()
                });
        let mut surface = create_surface(fb_info, &mut gr_context);
        let _scale = if let Ok(window) = self.window() {
            window.state().unwrap().scale
        } else {
            Scale::default()
        };

        let main_window = self.window().unwrap();
        let size = main_window.size().unwrap();
        let canvas = surface.canvas();
        if mode_size.1 > mode_size.0 {
            canvas.rotate(90., Some(skia_safe::Point::default()));//Some(skia_safe::Point::new(size.width as f32 / 2., 0.)));
            canvas.translate((0., -size.height as f32));
        }
        canvas.clear(Color::BLACK);
        canvas.flush();
        unsafe {
            swap_buffers();
        }
        let mut last_ts = Instant::now();
        let mut time = Duration::default();
        let mut frames_cnt = 0;
        loop {
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
            let now = Instant::now();
            main_window.run_timers(now);
            main_window.run_idle();

            let surface_canvas = surface.canvas();
            let main_window = self.window().unwrap();
            main_window.render(&mut *surface_canvas).unwrap();
            surface_canvas.flush();
            unsafe {
                swap_buffers();
            }
        }
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
