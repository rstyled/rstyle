//! One original-byte reconstruction per compiler source file and capture owner.
use rstyle_pdg::{ByteSpan, SourceSnapshot, SourceText, diagnostics::Stage};
use rustc_middle::ty::TyCtxt;
use rustc_span::{SourceFile, Span};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, Default)]
pub(crate) struct Sources(Rc<RefCell<BTreeMap<CompilerSource, Rc<Snapshot>>>>);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CompilerSource(u32);

pub(super) struct Snapshot {
    pub bytes: SourceSnapshot,
}

impl Snapshot {
    pub fn identity(&self) -> &str {
        self.bytes.identity()
    }
}

impl Sources {
    pub(super) fn file(&self, file: &SourceFile) -> Option<Rc<Snapshot>> {
        let key = CompilerSource(file.start_pos.0);
        let mut files = self.0.borrow_mut();
        if let Some(snapshot) = files.get(&key) {
            return Some(Rc::clone(snapshot));
        }

        let bytes = observed_recovery(file)?;
        let snapshot = Rc::new(Snapshot {
            bytes: bytes.into(),
        });
        files.insert(key, Rc::clone(&snapshot));
        Some(snapshot)
    }

    pub(super) fn range(&self, tcx: TyCtxt<'_>, span: Span) -> Option<(SourceText, ByteSpan)> {
        let file = tcx.sess.source_map().lookup_source_file(span.lo());
        if span.hi() > file.end_position() {
            return None;
        }

        let snapshot = self.file(&file)?;
        let range = ByteSpan {
            start: file.original_relative_byte_pos(span.lo()).0,
            end: file.original_relative_byte_pos(span.hi()).0,
        };
        let bytes = SourceText::from_snapshot(snapshot.bytes.clone(), range).ok()?;
        Some((bytes, range))
    }
}

fn observed_recovery(file: &SourceFile) -> Option<String> {
    Stage::new(
        "capture.source_restore",
        "compiler-file",
        file.src.as_ref().map(|src| src.as_bytes()),
    )
    .observe(|| {
        recover(file).ok_or_else(|| "Inconsistent compiler normalization positions".to_owned())
    })
    .ok()
}

fn recover(file: &SourceFile) -> Option<String> {
    if !matches!(file.original_relative_byte_pos(file.start_pos).0, 0 | 3) {
        return None;
    }
    restore(
        file.src.as_ref()?,
        file.normalized_pos.iter().map(|p| (p.pos.0, p.diff)),
    )
}

/// rustc removes only a leading BOM and CR in CRLF. Recover recorded bytes,
/// never reopen a path that may have changed since the compiler read it.
fn restore(
    normalized: &str,
    changes: impl DoubleEndedIterator<Item = (u32, u32)>,
) -> Option<String> {
    let mut bytes = normalized.as_bytes().to_vec();
    for (position, difference) in changes.rev() {
        if position == 0 && difference == 3 {
            bytes.splice(0..0, [0xef, 0xbb, 0xbf]);
        } else {
            let position = position.checked_sub(1)? as usize;
            if bytes.get(position) != Some(&b'\n') {
                return None;
            }
            bytes.insert(position, b'\r');
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_span::{
        FileName,
        source_map::{FilePathMapping, SourceMap},
    };

    #[test]
    fn original_compiler_files_are_recovered_once_per_capture_owner() {
        let map = SourceMap::new(FilePathMapping::empty());
        let original = "\u{feff}// π\r\nfn f() {}\r\n";
        let file = map.new_source_file(FileName::Custom("first".into()), original.into());
        let sources = Sources::default();
        let first = sources.file(&file).unwrap();
        let borrowed = sources.clone().file(&file).unwrap();
        assert_eq!(first.bytes.as_ref(), original);
        assert!(Rc::ptr_eq(&first, &borrowed));
        assert!(std::ptr::eq(first.identity(), borrowed.identity()));
        assert_eq!(sources.0.borrow().len(), 1);

        let independent = Sources::default().file(&file).unwrap();
        assert!(!Rc::ptr_eq(&first, &independent));
        assert_eq!(first.identity(), independent.identity());
        let edited = map.new_source_file(FileName::Custom("edited".into()), "fn f() {1}".into());
        let edited = sources.file(&edited).unwrap();
        assert_ne!(first.identity(), edited.identity());
        assert_eq!(sources.0.borrow().len(), 2);
    }

    #[test]
    fn normalization_recovery_rejects_inconsistent_recorded_positions() {
        assert_eq!(
            restore("a\nb\n", [(2, 1), (4, 2)].into_iter()).unwrap(),
            "a\r\nb\r\n"
        );
        assert_eq!(
            restore("π\n", [(0, 3), (3, 4)].into_iter()).unwrap(),
            "\u{feff}π\r\n"
        );
        assert!(restore("abc", [(2, 1)].into_iter()).is_none());
        assert!(restore("abc", [(0, 1)].into_iter()).is_none());
        assert!(restore("\n", [(4, 1)].into_iter()).is_none());
    }
}
