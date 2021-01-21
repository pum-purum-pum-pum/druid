use std::any::Any;
use std::ffi::OsString;

use instant::Instant;

use crate::kurbo::{Point, Rect, Size};
use crate::window::{FileDialogToken, IdleToken, TimerToken, WindowLevel};
use crate::window;
use crate::piet::PietText;
use crate::mouse::{Cursor, CursorDesc};
use crate::dialog::{FileDialogOptions, FileDialogType};
use crate::error::Error as ShellError;
use crate::scale::Scale;

//use super::menu::Menu;
use crate::hotkey::HotKey;

use anyhow::Error as AnyError;

pub trait WindowPlatform {
    // Maybe this should be removed. Maybe implementor can own RendererContext and we don't need
    // that then
    type RendererContext;
    type WindowHandle: WindowHandlePlatform;

    fn render(&self, renderer_ctx: &mut Self::RendererContext) -> Result<(), AnyError>;
    
    fn connect(&self, handle: Self::WindowHandle);
}

pub trait IdleHandlePlatform {
    fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static;

    fn add_idle_token(&self, token: IdleToken);
}

pub trait MenuPlatform {
    fn new() -> Self;

    fn new_for_popup() -> Self;
    
    fn add_dropdown(&mut self, _menu: Self, _text: &str, _enabled: bool);
    
    fn add_item(
        &mut self,
        _id: u32,
        _text: &str,
        _key: Option<&HotKey>,
        _enabled: bool,
        _selected: bool,
    ) {
        log::warn!("unimplemented");
    }

    fn add_separator(&mut self) {
        log::warn!("unimplemented");
    }
}

pub trait WindowHandlePlatform {
    type IdleHandle: IdleHandlePlatform;
    type Menu: MenuPlatform;
    
    fn show(&self);
    

    fn resizable(&self, _resizable: bool);
    

    fn show_titlebar(&self, _show_titlebar: bool);

    fn set_position(&self, _position: Point);

    fn set_level(&self, _level: WindowLevel);
    

    fn get_position(&self) -> Point;

    fn set_size(&self, _size: Size); 

    fn get_size(&self) -> Size;

    fn set_window_state(&self, _state: window::WindowState);

    fn get_window_state(&self) -> window::WindowState;

    fn handle_titlebar(&self, _val: bool);
    

    fn close(&self);

    fn bring_to_front_and_focus(&self);

    fn request_anim_frame(&self);

    fn invalidate_rect(&self, rect: Rect);
    
    fn invalidate(&self);

    fn text(&self) -> PietText;

    fn request_timer(&self, deadline: Instant) -> TimerToken;

    fn set_cursor(&mut self, _cursor: &Cursor);

    fn make_cursor(&self, _cursor_desc: &CursorDesc) -> Option<Cursor>; 

    fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken>;

    fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken>;

    fn file_dialog(
        &self,
        _ty: FileDialogType,
        _options: FileDialogOptions,
    ) -> Result<OsString, ShellError>;

    /// Get a handle that can be used to schedule an idle task.
    fn get_idle_handle(&self) -> Option<Self::IdleHandle>;

    /// Get the `Scale` of the window.
    fn get_scale(&self) -> Result<Scale, ShellError>;

    fn set_menu(&self, _menu: Self::Menu);

    fn show_context_menu(&self, _menu: Self::Menu, _pos: Point);
    
    fn set_title(&self, _title: impl Into<String>);
}
