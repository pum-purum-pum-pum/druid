// Copyright 2020 The Druid Authors#.
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

//! Window creation and management.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::BinaryHeap;
use std::ffi::OsString;
use std::panic::Location;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use instant::Instant;

use crate::{
    kurbo::{Point, Rect, Size, Vec2},
    piet::{Piet, PietText},
    Code,
};

use anyhow::Error as AnyError;
use glutin::dpi::{PhysicalPosition, PhysicalSize};
use glutin::event::{KeyboardInput, VirtualKeyCode};

use super::application::Application;
use super::error::Error;
use super::keycodes;
use super::menu::Menu;
use super::util::Timer;
use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType};
use crate::error::Error as ShellError;
use crate::keyboard::Modifiers;
use crate::scale::{Scalable, Scale, ScaledArea};

use crate::keyboard::KeyState;

use crate::mouse::{Cursor, CursorDesc, MouseButton, MouseButtons, MouseEvent};
use crate::region::Region;
use crate::window;
use crate::window::{FileDialogToken, IdleToken, TimerToken, WinHandler, WindowLevel};

pub struct Window {
    handler: RefCell<Box<dyn WinHandler>>,
    window_state: RefCell<WindowState>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    timer_queue: Mutex<BinaryHeap<Timer>>,
}

pub fn convert_mouse_button(mouse_button: glutin::event::MouseButton) -> Option<MouseButton> {
    match mouse_button {
        glutin::event::MouseButton::Left => Some(MouseButton::Left),
        glutin::event::MouseButton::Right => Some(MouseButton::Right),
        glutin::event::MouseButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

fn virtual_keycode_to_code(vkc: VirtualKeyCode) -> Code {
    use glutin::event::VirtualKeyCode::*;

    // TODO: Many of these keycodes map to shift + another key, for now they are
    //       left as `Code::Unidentified` - as are any keys that don't have a
    //       good mapping in the `Code` enum like Fn keys above 12.
    match vkc {
        Key1 => Code::Digit1,
        Key2 => Code::Digit2,
        Key3 => Code::Digit3,
        Key4 => Code::Digit4,
        Key5 => Code::Digit5,
        Key6 => Code::Digit6,
        Key7 => Code::Digit7,
        Key8 => Code::Digit8,
        Key9 => Code::Digit9,
        Key0 => Code::Digit0,
        A => Code::KeyA,
        B => Code::KeyB,
        C => Code::KeyC,
        D => Code::KeyD,
        E => Code::KeyE,
        F => Code::KeyF,
        G => Code::KeyG,
        H => Code::KeyH,
        I => Code::KeyI,
        J => Code::KeyJ,
        K => Code::KeyK,
        L => Code::KeyL,
        M => Code::KeyM,
        N => Code::KeyN,
        O => Code::KeyO,
        P => Code::KeyP,
        Q => Code::KeyQ,
        R => Code::KeyR,
        S => Code::KeyS,
        T => Code::KeyT,
        U => Code::KeyU,
        V => Code::KeyV,
        W => Code::KeyW,
        X => Code::KeyX,
        Y => Code::KeyY,
        Z => Code::KeyZ,
        Escape => Code::Escape,
        F1 => Code::F1,
        F2 => Code::F2,
        F3 => Code::F3,
        F4 => Code::F4,
        F5 => Code::F5,
        F6 => Code::F6,
        F7 => Code::F7,
        F8 => Code::F8,
        F9 => Code::F9,
        F10 => Code::F10,
        F11 => Code::F11,
        F12 => Code::F12,
        // TODO
        F13 | F14 | F15 | F16 | F17 | F18 | F19 | F20 | F21 | F22 | F23 | F24 => Code::Unidentified,
        Snapshot => Code::PrintScreen,
        Scroll => Code::ScrollLock,
        Pause => Code::Pause,
        Insert => Code::Insert,
        Home => Code::Home,
        Delete => Code::Delete,
        End => Code::End,
        PageDown => Code::PageDown,
        PageUp => Code::PageUp,
        Left => Code::ArrowLeft,
        Up => Code::ArrowUp,
        Right => Code::ArrowRight,
        Down => Code::ArrowDown,
        Back => Code::Backspace,
        Return => Code::Enter,
        Space => Code::Space,
        // TODO
        Compose | Caret => Code::Unidentified,
        Numlock => Code::NumLock,
        Numpad0 => Code::Numpad0,
        Numpad1 => Code::Numpad1,
        Numpad2 => Code::Numpad2,
        Numpad3 => Code::Numpad3,
        Numpad4 => Code::Numpad4,
        Numpad5 => Code::Numpad5,
        Numpad6 => Code::Numpad6,
        Numpad7 => Code::Numpad7,
        Numpad8 => Code::Numpad8,
        Numpad9 => Code::Numpad9,
        NumpadAdd => Code::NumpadAdd,
        NumpadDivide => Code::NumpadDivide,
        NumpadDecimal => Code::NumpadDecimal,
        NumpadComma => Code::NumpadComma,
        NumpadEnter => Code::NumpadEnter,
        NumpadEquals => Code::NumpadEqual,
        NumpadMultiply => Code::NumpadMultiply,
        NumpadSubtract => Code::NumpadSubtract,
        // TODO
        AbntC1 | AbntC2 | Apostrophe | Apps | Asterisk | At | Ax => Code::Unidentified,
        Backslash => Code::Backslash,
        // TODO
        Calculator | Capital | Colon => Code::Unidentified,
        Comma => Code::Comma,
        Convert => Code::Convert,
        Equals => Code::Equal,
        // TODO
        Grave | Kana | Kanji | LAlt | LBracket | LControl | LShift | LWin | Mail => {
            Code::Unidentified
        }
        MediaSelect => Code::MediaSelect,
        MediaStop => Code::MediaStop,
        Minus => Code::Minus,
        // TODO
        Mute | MyComputer | NavigateForward | NavigateBackward | NextTrack => Code::Unidentified,
        NoConvert => Code::NonConvert,
        OEM102 => Code::Unidentified,
        Period => Code::Period,
        // TODO
        PlayPause | Plus => Code::Unidentified,
        Power => Code::Power,
        // TODO
        PrevTrack | RAlt | RBracket | RControl | RShift | RWin => Code::Unidentified,
        Semicolon => Code::Semicolon,
        Slash => Code::Slash,
        Sleep => Code::Sleep,
        // TODO
        Stop | Sysrq => Code::Unidentified,
        Tab => Code::Tab,
        // TODO
        Underline | Unlabeled | VolumeDown | VolumeUp | Wake | WebBack | WebFavorites
        | WebForward | WebHome | WebRefresh | WebSearch | WebStop | Yen => Code::Unidentified,
        Copy => Code::Copy,
        Paste => Code::Paste,
        Cut => Code::Cut,
    }
}

impl Window {
    pub fn render(&self, canvas: &mut skia_safe::Canvas) -> Result<(), AnyError> {
        // important for AnimStart and invalidation of required regions
        self.with_handler(|h| h.prepare_paint());
        let invalid =
            std::mem::replace(&mut borrow_mut!(self.window_state)?.invalid, Region::EMPTY);
        let prev_invalid = borrow_mut!(self.window_state)?.prev_invalid.clone();
        let mut buffer_damage = invalid.clone();
        buffer_damage.union_with(&prev_invalid);
        canvas.save();
        let mut region = skia_safe::region::Region::new();
        for rect in buffer_damage.rects() {
            let scale = self.state()?.scale;
            let rect = rect.to_px(scale);
            let rect = skia_safe::IRect {
                left: rect.x0 as i32,
                top: rect.y0 as i32,
                right: rect.x1 as i32,
                bottom: rect.y1 as i32,
            };
            region.op_rect(rect, skia_safe::region::RegionOp::Union);
        }
        canvas.clip_region(&region, None);
        let mut piet_ctx = Piet::new(canvas);
        let mut win_handler = borrow_mut!(self.handler).unwrap();

        win_handler.paint(&mut piet_ctx, &buffer_damage);
        canvas.restore();

        borrow_mut!(self.window_state)?.prev_invalid = invalid;
        Ok(())
    }

    #[track_caller]
    fn with_handler<T, F: FnOnce(&mut dyn WinHandler) -> T>(&self, f: F) -> Option<T> {
        if self.handler.try_borrow_mut().is_err() || self.state_mut().is_err() {
            log::error!("other RefCells were borrowed when calling into the handler");
            return None;
        }

        self.with_handler_and_dont_check_the_other_borrows(f)
    }

    #[track_caller]
    fn with_handler_and_dont_check_the_other_borrows<T, F: FnOnce(&mut dyn WinHandler) -> T>(
        &self,
        f: F,
    ) -> Option<T> {
        match self.handler.try_borrow_mut() {
            Ok(mut h) => Some(f(&mut **h)),
            Err(_) => {
                log::error!("failed to borrow WinHandler at {}", Location::caller());
                None
            }
        }
    }

    pub fn connect(&self, handle: WindowHandle) -> Result<(), AnyError> {
        let size = self.size()?;
        let scale = self.state()?.scale;
        self.with_handler_and_dont_check_the_other_borrows(|h| {
            h.connect(&handle.into());
            h.scale(scale);
            h.size(size)
        });
        Ok(())
    }

    // TODO this is pub temporary cause we are using it to create window from application.rs
    pub fn size(&self) -> Result<Size, AnyError> {
        Ok(borrow!(self.window_state)?.size)
    }

    pub(crate) fn run_idle(&self) {
        let mut queue = Vec::new();
        std::mem::swap(&mut *self.idle_queue.lock().unwrap(), &mut queue);

        let mut needs_redraw = false;
        self.with_handler(|handler| {
            for callback in queue {
                match callback {
                    IdleKind::Callback(f) => {
                        f.call(handler.as_any());
                    }
                    IdleKind::Token(tok) => {
                        handler.idle(tok);
                    }
                    IdleKind::_Redraw => {
                        needs_redraw = true;
                    }
                }
            }
        });

        // TODO
        //if needs_redraw {
        //    if let Err(e) = self.redraw_now() {
        //        log::error!("Error redrawing: {}", e);
        //    }
        //}
    }

    pub(crate) fn next_timeout(&self) -> Option<Instant> {
        if let Some(timer) = self.timer_queue.lock().unwrap().peek() {
            Some(timer.deadline())
        } else {
            None
        }
    }

    pub(crate) fn run_timers(&self, now: Instant) {
        while let Some(deadline) = self.next_timeout() {
            if deadline > now {
                break;
            }
            // Remove the timer and get the token
            let token = self.timer_queue.lock().unwrap().pop().unwrap().token();
            self.with_handler(|h| h.timer(token));
        }
    }

    pub(crate) fn state_mut(&self) -> Result<std::cell::RefMut<WindowState>, AnyError> {
        borrow_mut!(self.window_state)
    }

    pub(crate) fn state(&self) -> Result<std::cell::Ref<WindowState>, AnyError> {
        borrow!(self.window_state)
    }

    pub fn screen_size_changed(&self, physical_size: PhysicalSize<u32>) -> Result<(), AnyError> {
        let scale = self.state()?.scale;
        let size = Size::new(physical_size.width as f64, physical_size.height as f64).to_dp(scale);

        self.state_mut()?.size = Size::new(physical_size.width as f64, physical_size.height as f64);
        self.with_handler(|h| h.size(size));
        Ok(())
    }

    pub fn handle_key_press(&self, key_press: KeyboardInput) {
        let state = match key_press.state {
            glutin::event::ElementState::Pressed => KeyState::Down,
            glutin::event::ElementState::Released => KeyState::Up,
        };
        let virtual_keycode = key_press.virtual_keycode;
        let code = virtual_keycode
            .map(virtual_keycode_to_code)
            .unwrap_or(Code::Unidentified);
        // TODO mods
        let mods = Modifiers::empty();
        // TODO location
        let location = crate::Location::Standard;
        let key = keycodes::code_to_key(code, mods);

        let key_event = crate::KeyEvent {
            code,
            key,
            mods,
            location,
            state,
            repeat: false,
            is_composing: false,
        };
        match state {
            KeyState::Down => {
                self.with_handler(|h| h.key_down(key_event));
            }
            KeyState::Up => {
                self.with_handler(|h| h.key_up(key_event));
            }
        }
    }

    pub fn handle_motion_notify(&self, physical_position: PhysicalPosition<f64>) {
        let scale = self.state().unwrap().scale; // TODO unwrap
        let mouse_event = MouseEvent {
            pos: Point::new(physical_position.x, physical_position.y).to_dp(scale),
            buttons: MouseButtons::new(),
            mods: Modifiers::empty(), // TODO
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: Vec2::ZERO,
        };
        self.with_handler(|h| h.mouse_move(&mouse_event));
    }

    pub fn handle_button_press(
        &self,
        physical_position: PhysicalPosition<f64>,
        mouse_button: glutin::event::MouseButton,
    ) {
        if let Some(button) = convert_mouse_button(mouse_button) {
            let scale = self.state().unwrap().scale; // TODO unwrap
            let mouse_event = MouseEvent {
                pos: Point::new(physical_position.x, physical_position.y).to_dp(scale),
                buttons: MouseButtons::new(),
                mods: Modifiers::empty(), // TODO
                count: 1,
                focus: false,
                button,
                wheel_delta: Vec2::ZERO,
            };
            self.with_handler(|h| h.mouse_down(&mouse_event));
        }
    }

    pub fn handle_button_release(
        &self,
        physical_position: PhysicalPosition<f64>,
        mouse_button: glutin::event::MouseButton,
    ) {
        if let Some(button) = convert_mouse_button(mouse_button) {
            let scale = self.state().unwrap().scale; // TODO unwrap
            let mouse_event = MouseEvent {
                pos: Point::new(physical_position.x, physical_position.y).to_dp(scale),
                buttons: MouseButtons::new(),
                mods: Modifiers::empty(), // TODO
                count: 0,
                focus: false,
                button,
                wheel_delta: Vec2::ZERO,
            };
            self.with_handler(|h| h.mouse_up(&mouse_event));
        }
    }
    //    pub fn handle_button_release(&self, button_release: &xproto::ButtonReleaseEvent) {
    //        let button = mouse_button(button_release.detail);
    //        let mouse_event = MouseEvent {
    //            pos: Point::new(button_release.event_x as f64, button_release.event_y as f64),
    //            // The xcb state includes the newly released button, but druid
    //            // doesn't want it.
    //            buttons: mouse_buttons(button_release.state).without(button),
    //            mods: key_mods(button_release.state),
    //            count: 0,
    //            focus: false,
    //            button,
    //            wheel_delta: Vec2::ZERO,
    //        };
    //        self.with_handler(|h| h.mouse_up(&mouse_event));
    //    }

    /// Schedule a redraw on the idle loop, or if we are waiting on present then schedule it for
    /// when the current present finishes.
    fn request_anim_frame(&self) {
        //if let Ok(true) = self.waiting_on_present() {
        //    if let Err(e) = self.set_needs_present(true) {
        //        log::error!(
        //            "Window::request_anim_frame - failed to schedule present: {}",
        //            e
        //        );
        //    }
        //} else {
        //    let idle = IdleHandle {
        //        queue: Arc::clone(&self.idle_queue),
        //        pipe: self.idle_pipe,
        //    };
        //    idle.schedule_redraw();
        //}
    }

    pub fn invalidate(&self) {
        match self.state().map(|state| state.size) {
            Ok(size) => self.invalidate_rect(size.to_rect()),
            Err(err) => log::error!("Window::invalidate - failed to get size: {}", err),
        }
    }

    pub fn invalidate_rect(&self, rect: Rect) {
        if let Err(err) = self.add_invalid_rect(rect) {
            log::error!("Window::invalidate_rect - failed to enlarge rect: {}", err);
        }
        self.request_anim_frame();
    }

    pub fn add_invalid_rect(&self, rect: Rect) -> Result<(), AnyError> {
        let scale = self.state()?.scale;
        let rect = rect.to_px(scale).expand().to_dp(scale);
        self.state_mut()?.invalid.add_rect(rect);
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct WindowHandle(Weak<Window>);

/// A handle that can get used to schedule an idle handler. Note that
/// this handle can be cloned and sent between threads.
#[derive(Clone)]
pub struct IdleHandle {
    queue: Arc<Mutex<Vec<IdleKind>>>,
    // TODO why there was file descriptor x11, what's the purpose of it
    // note that it's created in application.rs also
    //    pipe: RawFd,
}

pub(crate) enum IdleKind {
    Callback(Box<dyn IdleCallback>),
    Token(IdleToken),
    _Redraw,
}

impl IdleHandle {
    fn wake(&self) {
        //        loop {
        //            match nix::unistd::write(self.pipe, &[0]) {
        //                Err(nix::Error::Sys(nix::errno::Errno::EINTR)) => {}
        //                Err(nix::Error::Sys(nix::errno::Errno::EAGAIN)) => {}
        //                Err(e) => {
        //                    log::error!("Failed to write to idle pipe: {}", e);
        //                    break;
        //                }
        //                Ok(_) => {
        //                    break;
        //                }
        //            }
        //        }
    }

    pub(crate) fn _schedule_redraw(&self) {
        self.queue.lock().unwrap().push(IdleKind::_Redraw);
        self.wake();
    }

    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        self.queue
            .lock()
            .unwrap()
            .push(IdleKind::Callback(Box::new(callback)));
        self.wake();
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        self.queue.lock().unwrap().push(IdleKind::Token(token));
        self.wake();
    }
}

pub(crate) struct WindowState {
    pub(crate) scale: Scale,
    _area: Cell<ScaledArea>,
    _idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    size: Size,
    invalid: Region,
    prev_invalid: Region,
}

// TODO: support custom cursors
#[derive(Clone, PartialEq)]
pub struct CustomCursor;

/// Builder abstraction for creating new windows.
pub(crate) struct WindowBuilder {
    app: Application,
    handler: Option<Box<dyn WinHandler>>,
    _title: String,
    _cursor: Cursor,
    _menu: Option<Menu>,
    size: Size,
}

impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app,
            handler: None,
            _title: String::new(),
            _cursor: Cursor::Arrow,
            _menu: None,
            size: Size::new(800., 600.),
        }
    }

    /// This takes ownership, and is typically used with UiMain
    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn set_min_size(&mut self, _: Size) {
        // Ignored
    }

    pub fn resizable(&mut self, _resizable: bool) {
        // Ignored
    }

    pub fn show_titlebar(&mut self, _show_titlebar: bool) {
        // Ignored
    }

    pub fn set_position(&mut self, _position: Point) {
        // Ignored
    }

    pub fn set_window_state(&self, _state: window::WindowState) {
        // Ignored
    }

    pub fn set_level(&mut self, _level: WindowLevel) {
        // ignored
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self._title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self._menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        let handler = self.handler.unwrap();
        // TODO
        let state = WindowState {
            scale: Scale::new(2., 2.),
            _area: Cell::new(ScaledArea::default()),
            _idle_queue: Default::default(),
            size: self.size,
            invalid: Region::EMPTY,
            prev_invalid: Region::EMPTY,
        };
        let window = Rc::new(Window {
            handler: RefCell::new(handler),
            window_state: RefCell::new(state),
            idle_queue: Arc::new(Mutex::new(Vec::new())),
            timer_queue: Mutex::new(BinaryHeap::new()),
        });

        let handle = WindowHandle(Rc::downgrade(&window));
        window.connect(handle.clone()).unwrap();
        self.app.add_window(window).unwrap(); // TODO Vlad handle error here
        Ok(handle)
    }
}

impl WindowHandle {
    pub fn show(&self) {}

    pub fn resizable(&self, _resizable: bool) {
        log::warn!("resizable unimplemented for web");
    }

    pub fn show_titlebar(&self, _show_titlebar: bool) {
        log::warn!("show_titlebar unimplemented for web");
    }

    pub fn set_position(&self, _position: Point) {
        log::warn!("WindowHandle::set_position unimplemented for web");
    }

    pub fn set_level(&self, _level: WindowLevel) {
        log::warn!("WindowHandle::set_level  is currently unimplemented for web.");
    }

    pub fn get_position(&self) -> Point {
        log::warn!("WindowHandle::get_position unimplemented for web.");
        Point::new(0.0, 0.0)
    }

    pub fn set_size(&self, _size: Size) {
        log::warn!("WindowHandle::set_size unimplemented for web.");
    }

    pub fn get_size(&self) -> Size {
        log::warn!("WindowHandle::get_size unimplemented for web.");
        Size::new(0.0, 0.0)
    }

    pub fn set_window_state(&self, _state: window::WindowState) {
        log::warn!("WindowHandle::set_window_state unimplemented for web.");
    }

    pub fn get_window_state(&self) -> window::WindowState {
        log::warn!("WindowHandle::get_window_state unimplemented for web.");
        window::WindowState::RESTORED
    }

    pub fn handle_titlebar(&self, _val: bool) {
        log::warn!("WindowHandle::handle_titlebar unimplemented for web.");
    }

    pub fn close(&self) {
        // TODO
    }

    pub fn bring_to_front_and_focus(&self) {
        log::warn!("bring_to_frontand_focus unimplemented for web");
    }

    pub fn request_anim_frame(&self) {}

    pub fn invalidate_rect(&self, rect: Rect) {
        if let Some(window) = self.0.upgrade() {
            window.invalidate_rect(rect);
        }
        self.request_anim_frame();
    }

    pub fn invalidate(&self) {
        if let Some(window) = self.0.upgrade() {
            window.invalidate();
        }
    }

    pub fn text(&self) -> PietText {
        let _s = self
            .0
            .upgrade()
            .unwrap_or_else(|| panic!("Failed to produce a text context"));
        PietText::new()
    }

    pub fn request_timer(&self, deadline: Instant) -> TimerToken {
        if let Some(w) = self.0.upgrade() {
            let timer = Timer::new(deadline);
            w.timer_queue.lock().unwrap().push(timer);
            timer.token()
        } else {
            TimerToken::INVALID
        }
    }

    pub fn set_cursor(&mut self, _cursor: &Cursor) {}

    pub fn make_cursor(&self, _cursor_desc: &CursorDesc) -> Option<Cursor> {
        log::warn!("Custom cursors are not yet supported in the web backend");
        None
    }

    pub fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        log::warn!("open_file is currently unimplemented for web.");
        None
    }

    pub fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        log::warn!("save_as is currently unimplemented for web.");
        None
    }

    pub fn file_dialog(
        &self,
        _ty: FileDialogType,
        _options: FileDialogOptions,
    ) -> Result<OsString, ShellError> {
        Err(ShellError::Platform(Error::Unimplemented))
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        if let Some(w) = self.0.upgrade() {
            Some(IdleHandle {
                queue: Arc::clone(&w.idle_queue),
                //pipe: w.idle_pipe,
            })
        } else {
            None
        }
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        unimplemented!();
        //Ok(self
        //    .0
        //    .upgrade()
        //    .ok_or(ShellError::WindowDropped)?
        //    .scale
        //    .get())
    }

    pub fn set_menu(&self, _menu: Menu) {
        log::warn!("set_menu unimplemented for web");
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        log::warn!("show_context_menu unimplemented for web");
    }

    pub fn set_title(&self, _title: impl Into<String>) {
        unimplemented!();
        //log::warn!("set_title is not implemented");
    }
}

unsafe impl Send for IdleHandle {}

fn _mouse_button(button: i16) -> Option<MouseButton> {
    match button {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Middle),
        2 => Some(MouseButton::Right),
        3 => Some(MouseButton::X1),
        4 => Some(MouseButton::X2),
        _ => None,
    }
}

fn _mouse_buttons(mask: u16) -> MouseButtons {
    let mut buttons = MouseButtons::new();
    if mask & 1 != 0 {
        buttons.insert(MouseButton::Left);
    }
    if mask & 1 << 1 != 0 {
        buttons.insert(MouseButton::Right);
    }
    if mask & 1 << 2 != 0 {
        buttons.insert(MouseButton::Middle);
    }
    if mask & 1 << 3 != 0 {
        buttons.insert(MouseButton::X1);
    }
    if mask & 1 << 4 != 0 {
        buttons.insert(MouseButton::X2);
    }
    buttons
}
