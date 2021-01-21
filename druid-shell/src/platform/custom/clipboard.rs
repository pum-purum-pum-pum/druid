use crate::clipboard::{ClipboardFormat, FormatId};

pub trait ClipboardPlatform {
    /// Put a string onto the system clipboard.
    fn put_string(&mut self, _s: impl AsRef<str>);

    /// Put multi-format data on the system clipboard.
    fn put_formats(&mut self, _formats: &[ClipboardFormat]);

    /// Get a string from the system clipboard, if one is available.
    fn get_string(&self) -> Option<String>;

    /// Given a list of supported clipboard types, returns the supported type which has
    /// highest priority on the system clipboard, or `None` if no types are supported.
    fn preferred_format(&self, _formats: &[FormatId]) -> Option<FormatId>;

    /// Return data in a given format, if available.
    ///
    /// It is recommended that the `fmt` argument be a format returned by
    /// [`Clipboard::preferred_format`]
    fn get_format(&self, _format: FormatId) -> Option<Vec<u8>>;

    fn available_type_names(&self) -> Vec<String>;
}
