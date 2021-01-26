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

use super::application::ApplicationPlatform;
use crate::hotkey::HotKey;

use crate::platform::custom::error::PlatformError;

use anyhow::Error as AnyError;

pub trait WindowPlatform {
    // Maybe this should be removed. Maybe implementor can own RendererContext
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
    type CustomCursor;
    type Error: std::error::Error;
    
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

    fn set_cursor(&mut self, _cursor: &Cursor<Self::CustomCursor>);

    fn make_cursor(&self, _cursor_desc: &CursorDesc) -> Option<Cursor<Self::CustomCursor>>; 

    fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken>;

    fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken>;

    fn file_dialog(
        &self,
        _ty: FileDialogType,
        _options: FileDialogOptions,
    ) -> Result<OsString, ShellError<Self::Error>>;

    /// Get a handle that can be used to schedule an idle task.
    fn get_idle_handle(&self) -> Option<Self::IdleHandle>;

    /// Get the `Scale` of the window.
    fn get_scale(&self) -> Result<Scale, ShellError<Self::Error>>;

    fn set_menu(&self, _menu: Self::Menu);

    fn show_context_menu(&self, _menu: Self::Menu, _pos: Point);
    
    fn set_title(&self, _title: impl Into<String>);
}

pub trait WindowBuilderPlatform: Sized {
    type WindowHandle: WindowHandlePlatform;
    type Application: ApplicationPlatform;
    type Menu: MenuPlatform;
    type Error: PlatformError;
    /// Create a new `WindowBuilder`.
    ///
    /// Takes the [`Application`](crate::Application) that this window is for.
    fn new(app: Self::Application) -> Self;

    /// Set the [`WinHandler`] for this window.
    ///
    /// This is the object that will receive callbacks from this window.
    fn set_handler(&mut self, handler: Box<dyn window::WinHandler<Self::WindowHandle>>);

    /// Set the window's initial drawing area size in [display points](crate::Scale).
    ///
    /// The actual window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the size of the window.  The
    /// platform might choose a different size depending on its DPI or other platform-dependent
    /// configuration.  To know the actual size of the window you should handle the
    /// [`WinHandler::size`] method.
    fn set_size(&mut self, size: Size);

    /// Set the window's minimum drawing area size in [display points](crate::Scale).
    ///
    /// The actual minimum window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the minimum size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    fn set_min_size(&mut self, size: Size);

    /// Set whether the window should be resizable.
    fn resizable(&mut self, resizable: bool);

    /// Set whether the window should have a titlebar and decorations.
    fn show_titlebar(&mut self, show_titlebar: bool);

    /// Sets the initial window position in [pixels](crate::Scale), relative to the origin of the
    /// virtual screen.
    fn set_position(&mut self, position: Point);

    /// Sets the initial [`WindowLevel`].
    fn set_level(&mut self, level: WindowLevel);

    /// Set the window's initial title.
    fn set_title(&mut self, title: impl Into<String>);

    /// Set the window's menu.
    fn set_menu(&mut self, menu: Self::Menu);

    /// Sets the initial state of the window.
    fn set_window_state(&mut self, state: window::WindowState);

    /// Attempt to construct the platform window.
    ///
    /// If this fails, your application should exit.
    fn build(self) -> Result<Self::WindowHandle, Self::Error>;
}
