//! Source text of a unit: span slices, 1-based lines and character columns, and the
//! comment line above a function.

use super::model::{FileSpan, Unit};
use super::tree::lookup;
use crate::lines::preceding_comment_line;
use std::collections::BTreeMap;

/// Lines above a signature that are searched for its comment, as the rustc frontend does.
const LOOKBACK_LINES: usize = 40;

/// What a document says of a file beyond its text.
#[derive(Clone, Copy, Default)]
pub struct FileMarks {
    /// The build, rather than an author, wrote it.
    pub generated: bool,
    /// The language says it holds tests.
    pub test: bool,
}

pub struct Sources<'a> {
    files: BTreeMap<&'a str, (&'a str, &'a str)>,
    marks: BTreeMap<&'a str, FileMarks>,
}

impl<'a> Sources<'a> {
    /// Rule facts read function source, so every unit file must carry its content.
    pub fn new(unit: &'a Unit) -> Result<Self, String> {
        let mut files = BTreeMap::new();
        for file in &unit.files {
            let content = file.content.as_deref().ok_or_else(|| {
                format!(
                    "File {} has no content; rule facts need source text",
                    file.id
                )
            })?;
            if files
                .insert(file.id.as_str(), (file.path.as_str(), content))
                .is_some()
            {
                return Err(format!("Duplicate file ID {}", file.id));
            }
        }

        let marks = unit.files.iter().map(|file| {
            let marks = FileMarks {
                generated: file.origin == "generated",
                test: file.test,
            };
            (file.id.as_str(), marks)
        });
        Ok(Self {
            files,
            marks: marks.collect(),
        })
    }

    /// What the document says of the file a span lies in.
    pub fn marks(&self, span: &FileSpan) -> FileMarks {
        let marks = self.marks.get(span.file.as_str());
        marks.copied().unwrap_or_default()
    }

    fn file(&self, id: &str) -> Result<(&'a str, &'a str), String> {
        lookup(&self.files, id, || format!("Unknown file {id}"))
    }

    pub fn path(&self, span: &FileSpan) -> Result<&'a str, String> {
        Ok(self.file(&span.file)?.0)
    }

    /// The text from `start`'s beginning to `end`'s end, both in one file.
    pub fn between(&self, start: &FileSpan, end: &FileSpan) -> Result<&'a str, String> {
        let (_, content) = self.file(&start.file)?;
        if start.file != end.file {
            return Err(format!(
                "Spans in {} and {} do not form one range",
                start.file, end.file
            ));
        }
        content.get(start.start..end.end).ok_or_else(|| {
            format!(
                "Span {}..{} is not a character range of {}",
                start.start, end.end, start.file
            )
        })
    }

    /// 1-based line and 1-based character column of a span's start.
    pub fn location(&self, span: &FileSpan) -> Result<(usize, usize), String> {
        let (_, content) = self.file(&span.file)?;
        let before = content.get(..span.start).ok_or_else(|| {
            format!(
                "Offset {} is not a character boundary of {}",
                span.start, span.file
            )
        })?;

        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        let line = 1 + before.matches('\n').count();
        Ok((line, 1 + before[line_start..].chars().count()))
    }

    /// The comment line directly above `line` of the span's file, attributes skipped.
    pub fn preceding_line(&self, span: &FileSpan, line: usize) -> Result<String, String> {
        let (_, content) = self.file(&span.file)?;
        let first = line.saturating_sub(1 + LOOKBACK_LINES);
        let above: Vec<&str> = content
            .lines()
            .skip(first)
            .take(line.saturating_sub(1) - first)
            .collect();
        Ok(preceding_comment_line(&above, above.len()))
    }
}
