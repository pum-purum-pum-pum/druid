use std::rc::Rc;
use anyhow::Error;

use super::clipboard::ClipboardPlatform;
use super::window::WindowPlatform;

use crate::application::AppHandler;

pub trait PlatformApplication {
    type Window: WindowPlatform;
    type Clipboard: ClipboardPlatform;

    fn add_window(&self, window: Rc<Self::Window>) -> Result<(), Error>;

    fn window(&self) -> Result<Rc<Self::Window>, Error>;

    fn run(self, _handler: Option<Box<dyn AppHandler>>);

    fn run_inner(self) -> Result<(), Error>;

    fn quit(&self);

    fn clipboard(&self) -> Self::Clipboard;

    fn get_locale() -> String;
}
