use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use memmap2::MmapOptions;

use crate::config::Config;
use crate::highlight::{SyntaxChoice, SyntaxEngine};

#[derive(Debug)]
pub struct DocumentSet {
    pub docs: Vec<Document>,
    pub lines: Vec<LineRef>,
    line_number_width: usize,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub name: String,
    pub path: Option<PathBuf>,
    pub syntax: SyntaxChoice,
    backing: Backing,
    line_starts: Vec<usize>,
}

#[derive(Debug, Clone)]
enum Backing {
    Bytes(Arc<[u8]>),
    Mmap(Arc<memmap2::Mmap>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRef {
    pub doc: usize,
    pub local_line: usize,
    pub header: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineView<'a> {
    pub text: Cow<'a, str>,
    pub bytes: Cow<'a, [u8]>,
    pub doc: usize,
    pub local_line: usize,
    pub global_line: usize,
    pub header: bool,
}

impl DocumentSet {
    pub fn from_paths(paths: &[PathBuf], config: &Config) -> Result<Self> {
        let mut docs = Vec::new();
        let engine = SyntaxEngine::new(&config.theme)?;
        for path in paths {
            let doc = Document::load(path, config, &engine)?;
            docs.push(doc);
        }
        Ok(Self::from_documents(docs, config))
    }

    pub fn from_stdin(config: &Config) -> Result<Self> {
        let engine = SyntaxEngine::new(&config.theme)?;
        let doc = Document::from_stdin(config, &engine)?;
        Ok(Self::from_documents(vec![doc], config))
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line_number_width(&self) -> usize {
        self.line_number_width
    }

    pub fn line(&self, global_line: usize) -> Option<LineView<'_>> {
        let line_ref = self.lines.get(global_line)?;
        let doc = &self.docs[line_ref.doc];
        if line_ref.header {
            let text = format!("==> {} <==", doc.name);
            Some(LineView {
                text: Cow::Owned(text.clone()),
                bytes: Cow::Owned(text.into_bytes()),
                doc: line_ref.doc,
                local_line: 0,
                global_line,
                header: true,
            })
        } else {
            let bytes = doc.line_bytes(line_ref.local_line)?;
            let text = doc.line_text(line_ref.local_line)?;
            Some(LineView {
                text,
                bytes: Cow::Borrowed(bytes),
                doc: line_ref.doc,
                local_line: line_ref.local_line,
                global_line,
                header: false,
            })
        }
    }

    pub fn document(&self, index: usize) -> Option<&Document> {
        self.docs.get(index)
    }

    pub fn document_index_at_line(&self, global_line: usize) -> Option<usize> {
        self.lines.get(global_line).map(|line_ref| line_ref.doc)
    }

    pub fn first_line_for_document(&self, doc_index: usize) -> Option<usize> {
        self.lines
            .iter()
            .position(|line_ref| line_ref.doc == doc_index)
    }

    pub fn first_visible_line_for_document(&self, doc_index: usize) -> Option<usize> {
        self.lines
            .iter()
            .position(|line_ref| line_ref.doc == doc_index && !line_ref.header)
            .or_else(|| self.first_line_for_document(doc_index))
    }

    pub fn reloaded(&self, config: &Config) -> Result<Self> {
        let engine = SyntaxEngine::new(&config.theme)?;
        let mut docs = Vec::with_capacity(self.docs.len());
        for doc in &self.docs {
            if let Some(path) = &doc.path {
                docs.push(Document::load(path, config, &engine)?);
            } else {
                docs.push(doc.clone());
            }
        }
        Ok(Self::from_documents(docs, config))
    }

    fn from_documents(docs: Vec<Document>, config: &Config) -> Self {
        let mut lines = Vec::new();
        let show_headers = docs.len() > 1;
        for (doc_index, doc) in docs.iter().enumerate() {
            let mut previous_blank = false;
            if show_headers {
                lines.push(LineRef {
                    doc: doc_index,
                    local_line: 0,
                    header: true,
                });
                previous_blank = false;
            }
            for local_line in 0..doc.line_count() {
                if config.squeeze_blank_lines {
                    let is_blank = doc
                        .line_bytes(local_line)
                        .map(|bytes| {
                            if config.raw_control_chars {
                                bytes.is_empty()
                            } else {
                                SyntaxEngine::strip_ansi_sequences(bytes).is_empty()
                            }
                        })
                        .unwrap_or(false);
                    if is_blank && previous_blank {
                        continue;
                    }
                    previous_blank = is_blank;
                }
                lines.push(LineRef {
                    doc: doc_index,
                    local_line,
                    header: false,
                });
            }
        }
        let line_number_width = docs
            .iter()
            .map(|doc| doc.line_count().max(1).to_string().len())
            .max()
            .unwrap_or(1);
        Self {
            docs,
            lines,
            line_number_width,
        }
    }
}

impl Document {
    pub fn load(path: &Path, config: &Config, engine: &SyntaxEngine) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let metadata = file.metadata()?;
        let backing = if metadata.is_file() && metadata.len() > 0 {
            // SAFETY: we keep the file mapped only for read access and own the mapping.
            let mmap = unsafe { MmapOptions::new().map(&file) }
                .with_context(|| format!("memory-mapping {}", path.display()))?;
            Backing::Mmap(Arc::new(mmap))
        } else {
            let mut buf = Vec::new();
            let mut reader = io::BufReader::new(file);
            reader
                .read_to_end(&mut buf)
                .with_context(|| format!("reading {}", path.display()))?;
            Backing::Bytes(Arc::from(buf.into_boxed_slice()))
        };
        Ok(Self::from_backing(
            path.display().to_string(),
            Some(path.to_path_buf()),
            backing,
            config,
            engine,
        ))
    }

    pub fn from_stdin(config: &Config, engine: &SyntaxEngine) -> Result<Self> {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).context("reading stdin")?;
        Ok(Self::from_backing(
            "<stdin>".to_string(),
            None,
            Backing::Bytes(Arc::from(buf.into_boxed_slice())),
            config,
            engine,
        ))
    }

    fn from_backing(
        name: String,
        path: Option<PathBuf>,
        backing: Backing,
        config: &Config,
        engine: &SyntaxEngine,
    ) -> Self {
        let bytes = backing.as_slice();
        let mut line_starts = Vec::new();
        if !bytes.is_empty() {
            line_starts.push(0);
            for (idx, b) in bytes.iter().enumerate() {
                if *b == b'\n' && idx + 1 < bytes.len() {
                    line_starts.push(idx + 1);
                }
            }
        }
        let syntax = engine.detect(&path, bytes, config.language.as_deref());
        Self {
            name,
            path,
            syntax,
            backing,
            line_starts,
        }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line_bytes(&self, line: usize) -> Option<&[u8]> {
        let start = *self.line_starts.get(line)?;
        let end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1].saturating_sub(1)
        } else if self.backing.as_slice().last() == Some(&b'\n') {
            self.backing.as_slice().len().saturating_sub(1)
        } else {
            self.backing.as_slice().len()
        };
        Some(&self.backing.as_slice()[start..end])
    }

    pub fn line_text(&self, line: usize) -> Option<Cow<'_, str>> {
        self.line_bytes(line)
            .map(|bytes| String::from_utf8_lossy(bytes))
    }
}

impl Backing {
    fn as_slice(&self) -> &[u8] {
        match self {
            Backing::Bytes(bytes) => bytes.as_ref(),
            Backing::Mmap(map) => map.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn splits_lines_without_extra_blank_line() {
        let engine = SyntaxEngine::new("base16-ocean.dark").unwrap();
        let config = Config::default();
        let doc = Document::from_backing(
            "demo".to_string(),
            None,
            Backing::Bytes(Arc::from(b"a\nb\n".as_slice())),
            &config,
            &engine,
        );
        assert_eq!(doc.line_count(), 2);
        assert_eq!(doc.line_text(0).unwrap(), "a");
        assert_eq!(doc.line_text(1).unwrap(), "b");
    }

    #[test]
    fn adds_headers_for_multiple_files() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("first.rs");
        let second = tmp.path().join("second.rs");
        std::fs::write(&first, "fn main() {}\n").unwrap();
        std::fs::write(&second, "let x = 1;\n").unwrap();
        let set = DocumentSet::from_paths(&[first, second], &Config::default()).unwrap();
        assert!(set.line(0).unwrap().header);
        assert!(set.line(2).unwrap().header);
        assert_eq!(set.line_count(), 4);
        assert_eq!(set.line_number_width(), 1);
    }

    #[test]
    fn finds_first_visible_line_for_each_document() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("first.rs");
        let second = tmp.path().join("second.rs");
        std::fs::write(&first, "fn main() {}\n").unwrap();
        std::fs::write(&second, "").unwrap();
        let set = DocumentSet::from_paths(&[first, second], &Config::default()).unwrap();

        assert_eq!(set.first_visible_line_for_document(0), Some(1));
        assert_eq!(set.first_visible_line_for_document(1), Some(2));
        assert_eq!(set.document_index_at_line(0), Some(0));
    }

    #[test]
    fn tracks_line_number_width_for_longer_files() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("first.rs");
        let second = tmp.path().join("second.rs");
        std::fs::write(&first, "a\n").unwrap();
        std::fs::write(&second, "b\n".repeat(120)).unwrap();
        let set = DocumentSet::from_paths(&[first, second], &Config::default()).unwrap();
        assert_eq!(set.line_number_width(), 3);
    }

    #[test]
    fn squeezes_consecutive_blank_lines_when_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("blank.rs");
        std::fs::write(&path, "alpha\n\n\nbeta\n").unwrap();
        let config = Config {
            squeeze_blank_lines: true,
            ..Config::default()
        };
        let set = DocumentSet::from_paths(&[path], &config).unwrap();
        assert_eq!(set.line_count(), 3);
        assert_eq!(set.line(0).unwrap().text, "alpha");
        assert_eq!(set.line(1).unwrap().text, "");
        assert_eq!(set.line(2).unwrap().text, "beta");
    }
}
