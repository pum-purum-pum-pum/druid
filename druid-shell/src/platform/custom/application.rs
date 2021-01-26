use std::rc::Rc;
use anyhow::Error;

use super::clipboard::ClipboardPlatform;
use super::window::WindowPlatform;

use crate::application::AppHandler;
use crate::platform::custom::error::PlatformError;

pub trait ApplicationPlatform: Sized {
    type Window: WindowPlatform;
    type Clipboard: ClipboardPlatform;
    // platform specific error type. We place it here to make sure that the same time used for all
    // traits in Platform (TODO review if it's true at the end)
    type Error: PlatformError;

    fn new() -> Result<Self, Error>;
    
    fn add_window(&self, window: Rc<Self::Window>) -> Result<(), Error>;

    fn window(&self) -> Result<Rc<Self::Window>, Error>;

    fn run(self, _handler: Option<Box<dyn AppHandler>>);

    fn run_inner(self) -> Result<(), Error>;

    fn quit(&self);

    fn clipboard(&self) -> Self::Clipboard;

    fn get_locale() -> String;
}
