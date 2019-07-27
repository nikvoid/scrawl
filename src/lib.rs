//! # Scrawl 
//! A library for opening a file for editing in a text editor and capturing the result as a String
#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unstable_features, unsafe_code,
        unused_import_braces, unused_qualifications)]

/* Standard Library */
use std::{
    fs,
    env::{temp_dir, var},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    process::Command
};

/* Internal Modules */
pub mod error;
use error::ScrawlError as ScrawlError;

/* Constants used by the struct to prevent naming collisions of buffer */
const PREFIX: &str = "xvrqt_scrawl";
static TEMP_FILE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// The Editor struct allows setting up the editor before opening it. Useful for setting things like a file extension for syntax highlighting, or specifying a specific editor and more.
#[derive(Debug)]
pub struct Editor {
    /// The name of the command to use instead of $EDITOR, fallsback to the user's default editor
    editor: Option<String>,
    /// Use the contents of specified file to seed the buffer.
    file: Option<PathBuf>,
    /// Use the contents of this String slice to seed the buffer.
    content: Option<String>,
    /// The extension to set on the file used a temporary buffer. Useful for having the correct syntax highlighting when the editor is opened.
    extension: Option<String>,

    /// Trim the white space off the resulting string. True by default.
    trim: bool,
    /// If file is set this will enable the user to directly edit the file that is opened. If 'file' is not set then this flag is ignored. False by default.
    edit_directly: bool,
}

impl Editor {
    /// Returns a new Editor struct with Trim Newlines and Require Save enabled. 
    pub fn new() -> Self {
        Editor {
            editor: None,

            file: None,
            content: None,
            extension: None,

            trim: true,
            edit_directly: false,
        }
    }

    /* SETTERS */

    /// Sets the name of the editor to open the text buffer. If this editor is not found it will not fallback on the user's default and return an error instead.
    /// # Example
    /// ```no_run
    /// fn main() {
    ///     let output = match scrawl::Editor.new()
    ///                                .executable("vim")
    ///                                .edit() 
    ///     {
    ///          Ok(s) => s,
    ///          Err(e) => e.to_string()
    ///     };
    ///     println!("{}", output);
    /// }
    /// ```
    pub fn editor(&mut self, command: &str) -> &mut Editor {
        self.editor= Some(command.to_owned());        
        self
    }

    /// Seeds the text buffer with the contents of the specified file. This does **not** edit the contents of the file unless 'edit_directly(true)' is set.
    pub fn file(&mut self, file: &Path) -> &mut Editor {
        self.file = Some(file.to_owned());        
        self
    }

    /// Fills the text buffer with the contents of the specified string. If both 'file' and 'contents' are set, contents will take priority. 
    pub fn contents(&mut self, contents: &str) -> &mut Editor {
        self.content = Some(contents.to_owned());        
        self
    }

    /// Sets the extension of the temporary file used as a buffer. Useful for hinting to the editor which syntax highlighting to use.
    pub fn extension(&mut self, ext: &str) -> &mut Editor {
        self.extension = Some(ext.to_owned());
        self
    }

    /// Sets whether or not to trim the resulting String of whitespace
    pub fn trim(&mut self, b: bool) -> &mut Editor {
        self.trim = b;
        self
    }

    /// Sets whether or not to save changes to the file specified in 'file'.
    pub fn edit_directly(&mut self, b: bool) -> &mut Editor {
        self.edit_directly = b;
        self
    }

    /* Utility */

    /* Opens the file in the user's preferred text editor, and returns the
     * contents as a String.
    */
    fn open_editor(&self) -> Result<String, ScrawlError> {
        let editor_name = self.get_editor_name();
        let path = self.get_file()?;

        match Command::new(&editor_name)
            .arg(&path)
            .status() { 
                Ok(status) if status.success() => {
                    fs::read_to_string(path).map_err(|_| {
                        ScrawlError::FailedToReadIntoString
                    })
                },
                _ => Err(ScrawlError::FailedToOpenEditor(editor_name))
        }
    }

    /* Attempts to determine which text editor to open the text buffer with. */
    fn get_editor_name(&self) -> String {
        /* Use the editor set by the caller */
        if let Some(ref editor) = self.editor { return editor.to_owned() }

        /* Check env vars for a default editor */
        if let Ok(editor) = var("VISUAL").or(var("EDITOR")) { return editor }
        
        /* Take a guess based on the system */
        if cfg!(windows) {
            String::from("notepad.exe")
        } else {
            String::from("vi")
        }
    }

    /* Returns the file to use as a buffer. Copies data to it if required */
    fn get_file(&self) -> Result<PathBuf, ScrawlError> {
        match self.file { 
            Some(ref path) if self.edit_directly => Ok(path.to_owned()), 
            _ => {
                /* Create a tempfile to use a buffer */
                let tempfile = self.create_temp_file()?;

                /* Seed the tempfile with content (if any) */
                if let Some(ref content) = self.content {
                    fs::write(&tempfile, content).map_err(|_| {
                        ScrawlError::FailedToCopyToTempFile("[String]".into())
                    })?;
                } else if let Some(ref path) = self.file {
                    fs::copy(path, &tempfile).map_err(|_| {
                        let path = path.to_str().unwrap_or("<unknown>");
                        ScrawlError::FailedToCopyToTempFile(String::from(path))
                    })?;
                }

                Ok(tempfile)
            }
        }
    }

    /* Creates a thread safe, process safe tempfile to use as a buffer */
    fn create_temp_file(&self) -> Result<PathBuf, ScrawlError> {
        /* Generate unique path to a temporary file buffer */
        let process_id = std::process::id();
        let i = TEMP_FILE_COUNT.fetch_add(1, Ordering::SeqCst);
        let ext = self.extension.as_ref().map_or("", AsRef::as_ref);
        let temp_file = format!("{}_{}_{}{}", PREFIX, process_id, i, ext);

        /* Push the file to the OS's temp dir */
        let mut temp_dir = temp_dir();
        temp_dir.push(temp_file);

        /* Create the file */
        fs::File::create(&temp_dir).map_err(|_e| {
            ScrawlError::FailedToCreateTempfile
        })?;

        Ok(temp_dir)
    }


    /// Opens a text editor with the settings in the struct. Returns a Result with the String upon success.
    pub fn edit(&self) -> Result<String, ScrawlError> {
        let mut output = self.open_editor()?;

        if self.trim {
            output = output.trim().to_owned();
        }

        Ok(output)
    }
}

/* Builder */

/// Creates a new Editor struct. Used to indicate you're not saving the struct for resuse.
/// # Example
/// ```no_run
/// fn main() {
///     let output = match scrawl::builder()
///                                .editor("vim") {
///          Ok(s) => s,
///          Err(e) => e.to_string()
///    };
///    println!("{}", output);
/// }
/// ```
pub fn builder() -> Editor {
    Editor::new()
}

/* Convenience functions */

/// New opens an empty text buffer in an editor and returns a Result<String> with the contents.
///
/// # Example
/// ```no_run
/// fn main() {
///     let output = match scrawl::new() {
///          Ok(s) => s,
///          Err(e) => e.to_string()
///    };
///    println!("{}", output);
/// }
/// ```
pub fn new() -> Result<String, ScrawlError> {
    builder().edit()
}

/// New opens an text buffer with the contents of the provided String in an editor. Returns a Result<String> with the edited contents.
///
/// # Example
/// ```no_run
/// fn main() {
///     let output = match scrawl::with("Hello World!") {
///          Ok(s) => s,
///          Err(e) => e.to_string()
///    };
///    println!("{}", output);
/// }
/// ```
pub fn with(content: &str) -> Result<String, ScrawlError> {
    builder().contents(content).edit()
}

/// Open opens a text buffer in an editor with the contents of the file specified. This does **not** edit the contents of the file. Returns a Result<String> with the contents of the buffer.
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// 
/// fn main() {
///     let path = Path::new("hello.txt");
///     let output = match scrawl::open(path) {
///          Ok(s) => s,
///          Err(e) => e.to_string()
///    };
///    println!("{}", output);
/// }
/// ```
pub fn open(p: &Path) -> Result<String, ScrawlError> {
    builder().file(p).edit()
}

/// Edit opens a text buffer in an editor with the contents of the file specified. This **does** edit the contents of the file. Returns a Result<String> with the contents of the buffer.
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// 
/// fn main() {
///     let path = Path::new("hello.txt");
///     let output = match scrawl::edit(path) {
///          Ok(s) => s,
///          Err(e) => e.to_string()
///    };
///    println!("{}", output);
/// }
/// ```
pub fn edit(p: &Path) -> Result<String, ScrawlError> {
    builder().file(p).edit_directly(true).edit()
}

