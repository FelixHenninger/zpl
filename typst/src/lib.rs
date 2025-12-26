use std::{path::PathBuf, sync::Arc};

use typst::{
    Library, World,
    diag::FileError,
    foundations::{Bytes, Datetime, Dict, Value},
    syntax::{FileId, Source, VirtualPath},
    text::{Font, FontBook},
    utils::LazyHash,
};

/// A `World` that tries not to depend on not-explicitly-configured host details.
///
/// Uses the `ZplHost` for all static details and then adds the template file / inputs to it.
pub struct ZplWorld {
    host: Arc<ZplHost>,
    main: FileId,
    main_source: Source,
    library: LazyHash<Library>,
}

pub struct ZplHost {
    fonts: Vec<Font>,
    font_book: LazyHash<FontBook>,
    root: Option<PathBuf>,
}

#[derive(serde::Serialize, Clone)]
pub struct PrinterLabel {
    /// Width of the label in mm.
    pub width: f32,
    /// Height of the label in mm.
    pub height: f32,
    /// Space to reserve on the left of the label, as by printed direction.
    pub margin_left: f32,
    /// Space to reserve on the right of the label, as by printed direction.
    pub margin_right: f32,
    /// Space to reserve on top of the label, as by printed direction.
    pub margin_top: f32,
    /// Space to reserve at the bottom of the label, as by printed direction.
    pub margin_bottom: f32,
}

impl World for ZplWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.host.font_book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> Result<Source, FileError> {
        eprintln!("Loading source: {id:?}");
        if id == self.main {
            return Ok(self.main_source.clone());
        }

        let Some(root) = &self.host.root else {
            return Err(FileError::AccessDenied);
        };

        let vpath = id.vpath().as_rootless_path();
        let abspath = root.join(vpath);

        eprintln!("Okay: {}", abspath.display());
        let data = std::fs::read_to_string(&abspath)
            .map_err(|io| FileError::from_io(io, vpath))?;

        eprintln!("Okay: {id:?}");
        return Ok(Source::new(id, data));
    }

    fn file(&self, id: FileId) -> Result<Bytes, FileError> {
        eprintln!("Loading file: {id:?}");
        if id == self.main {
            return Ok(Bytes::from_string(self.main_source.text().to_owned()));
        }

        let Some(root) = &self.host.root else {
            return Err(FileError::AccessDenied);
        };

        let vpath = id.vpath().as_rootless_path();
        let abspath = root.join(vpath);
        let data = std::fs::read(&abspath)
            .map_err(|io| FileError::from_io(io, vpath))?;

        return Ok(Bytes::new(data));
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.host.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        if offset.is_some() {
            return None;
        }

        let now = time::UtcDateTime::now();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
        Some(Datetime::Datetime(now))
    }
}

impl ZplHost {
    pub fn builder() -> ZplHost {
        let mut fonts = vec![];

        if let Ok(tf) =
            std::fs::read("/usr/share/fonts/gsfonts/NimbusSans-Regular.otf")
        {
            let font_bytes = Bytes::new(tf);
            fonts.extend(Font::iter(font_bytes));
        } else {
            eprintln!(
                "Could not load NimbusSans-Regular.otf from system fonts, we're running without text rendering"
            );
        }

        ZplHost {
            font_book: LazyHash::new(FontBook::from_fonts(fonts.iter())),
            fonts,
            root: None,
        }
    }

    pub fn with_root(self, root: PathBuf) -> ZplHost {
        ZplHost {
            root: Some(root),
            ..self
        }
    }

    pub fn build(self) -> Arc<Self> {
        Arc::new(self)
    }

    pub fn new() -> Arc<Self> {
        Self::builder().build()
    }

    pub fn instantiate(
        self: Arc<Self>,
        template: String,
        printer: PrinterLabel,
    ) -> ZplWorld {
        let library = LazyHash::new({
            let label = serde_json::to_string(&printer).unwrap();

            let dict =
                Dict::from_iter([("label".into(), Value::Str(label.into()))]);

            let mut builder = <Library as typst::LibraryExt>::builder();
            builder = builder.with_inputs(dict);
            builder.build()
        });

        let main = FileId::new_fake(VirtualPath::new("/main"));
        let main_source = Source::new(main, template);

        ZplWorld {
            host: self,
            library,
            main,
            main_source,
        }
    }
}

impl ZplWorld {
    pub fn render_to_svg_pages(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        struct CompilationFailed;

        impl core::fmt::Display for CompilationFailed {
            fn fmt(
                &self,
                f: &mut core::fmt::Formatter<'_>,
            ) -> core::fmt::Result {
                write!(f, "Compilation failed, consult console")
            }
        }

        impl core::fmt::Debug for CompilationFailed {
            fn fmt(
                &self,
                f: &mut core::fmt::Formatter<'_>,
            ) -> core::fmt::Result {
                write!(f, "{}", self)
            }
        }

        impl core::error::Error for CompilationFailed {}

        let doc = match typst::compile::<typst::layout::PagedDocument>(self) {
            typst::diag::Warned {
                output: Ok(doc), ..
            } => doc,
            typst::diag::Warned {
                output: Err(err),
                warnings,
            } => {
                eprintln!("Warnings: {warnings:#?}");
                eprintln!("Error: {err:#?}");
                return Err(Box::new(CompilationFailed));
            }
        };

        let mut svgs = vec![];

        for (_i, page) in doc.pages.iter().enumerate() {
            svgs.push(typst_svg::svg(page));
        }

        Ok(svgs)
    }
}
