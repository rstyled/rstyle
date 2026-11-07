use super::*;
use serde_json::{Value, json};
use std::io::Cursor;

#[test]
fn records_cross_arbitrary_chunks_and_finish_only_after_the_terminal_and_eof() {
    let bytes = b"\x1e{\"source\":\"\xce\xbb\\u001e\"}\n\x1e{\"second\":2}\n\x1enull\n";
    for capacity in [1, 2, 7, 8192] {
        let mut reader = SequenceReader::new(BufReader::with_capacity(capacity, bytes.as_slice()));
        assert!(reader.complete().is_err());
        assert_eq!(reader.next().unwrap().unwrap(), json!({"source":"λ\u{1e}"}));
        assert_eq!(reader.progress().received_documents, 1);
        assert_eq!(reader.next().unwrap().unwrap(), json!({"second":2}));
        assert!(reader.complete().is_err());
        assert!(reader.next().is_none());
        reader.complete().unwrap();
        assert_eq!(reader.progress().received_bytes, bytes.len() as u64);
        assert!(reader.progress().complete);
        assert!(reader.next().is_none());
    }
}

#[test]
fn invalid_or_uncommitted_streams_fail_without_accepting_a_prefix() {
    for bytes in [
        b"".as_slice(),
        b"{}\x1enull\n",
        b"\x1enull\n",
        b"\x1e{}\n",
        b"\x1e{}\n\x1e",
        b"\x1e{}\n\x1enul",
        b"\x1e{}\n\x1enull\n\x1e{}",
        b"\x1e{}\n\x1enull garbage",
        b"\x1e{}\n\x1enull\xc2\xa0",
        b"\x1e{} {}\x1enull\n",
        b"\x1e[]\x1enull\n",
        b"\x1e { }\x1enull\n",
        b"\x1e{\x1enull\n",
        b"\x1e{\"bad\":\"\xff\"}\x1enull\n",
    ] {
        let mut reader = SequenceReader::new(BufReader::with_capacity(2, bytes));
        assert!(
            reader.by_ref().collect::<Result<Vec<_>, _>>().is_err(),
            "{bytes:?}"
        );
        assert!(reader.complete().is_err(), "{bytes:?}");
        assert!(!reader.progress().complete);
        assert!(reader.next().is_none());
    }
}

#[test]
fn storage_callbacks_keep_original_bytes_without_a_json_tree() {
    let bytes = b"\x1e{ \"x\": 1 }\n\x1enull\n";
    let mut reader = SequenceReader::new(bytes.as_slice());
    let original = reader
        .next_with(|record| {
            let mut bytes = Vec::new();
            record
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            Ok(bytes)
        })
        .unwrap()
        .unwrap();
    assert_eq!(original, b"{ \"x\": 1 }\n");
    assert!(reader.next().is_none());
    reader.complete().unwrap();

    let mut reader = SequenceReader::new(bytes.as_slice());
    assert!(reader.next_with(|_| Ok(())).is_err());
    assert!(reader.complete().is_err());
}

struct Broken;

#[test]
fn typed_readers_validate_utf8_even_in_ignored_values() {
    let mut reader = SequenceReader::new(b"\x1e{\"bad\":\"\xff\"}\n\x1enull\n".as_slice());
    assert!(reader.next_document::<serde::de::IgnoredAny>().is_err());
    assert!(reader.complete().is_err());
}

impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::ConnectionReset,
            "producer disconnected",
        ))
    }
}

#[test]
fn a_read_error_after_the_completion_marker_still_rejects_the_stream() {
    let input = Cursor::new(b"\x1e{}\n\x1enull\n").chain(Broken);
    let mut reader = SequenceReader::new(BufReader::with_capacity(2, input));
    assert_eq!(reader.next_document::<Value>().unwrap(), Some(json!({})));
    let error = reader.next_document::<Value>().unwrap_err();
    assert!(error.contains("producer disconnected"), "{error}");
    assert_eq!(reader.progress().received_documents, 1);
    assert!(reader.complete().is_err());
}

fn sealed_documents(encoding: Encoding) -> (tempfile::TempDir, Vec<SealedDocument>) {
    let directory = tempfile::tempdir().unwrap();
    let seals = [json!({"source":"λ\u{1e}"}), json!({"second":2})]
        .iter()
        .enumerate()
        .map(|(index, document)| {
            let path = directory
                .path()
                .join(format!("document-{index:06}{}", encoding.suffix()));
            super::super::write_json(&path, encoding, document).unwrap()
        })
        .collect();
    (directory, seals)
}

#[test]
fn sealed_documents_send_as_a_sequence_a_reader_accepts() {
    for encoding in [Encoding::Json, Encoding::Zstd] {
        let (directory, seals) = sealed_documents(encoding);
        let mut sent = Vec::new();
        send_sealed(directory.path(), &seals, &mut sent).unwrap();

        let mut reader = SequenceReader::new(sent.as_slice());
        assert_eq!(reader.next().unwrap().unwrap(), json!({"source":"λ\u{1e}"}));
        assert_eq!(reader.next().unwrap().unwrap(), json!({"second":2}));
        assert!(reader.next().is_none());
        reader.complete().unwrap();
    }
}

#[test]
fn a_document_that_disagrees_with_its_seal_leaves_a_sequence_a_reader_refuses() {
    for encoding in [Encoding::Json, Encoding::Zstd] {
        let (directory, mut seals) = sealed_documents(encoding);
        let last = seals.last_mut().unwrap();
        last.decoded_checksum = Some(format!("sha256:{}", "0".repeat(64)));
        let mut sent = Vec::new();
        let error = send_sealed(directory.path(), &seals, &mut sent).unwrap_err();
        assert!(error.contains("disagrees with its manifest"), "{error}");

        let mut reader = SequenceReader::new(sent.as_slice());
        let read: Result<Vec<_>, _> = reader.by_ref().collect();
        assert!(read.is_err() || reader.complete().is_err(), "{sent:?}");
        assert!(reader.complete().is_err());
    }
}

impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "the reader stopped reading",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_reader_that_stops_stops_the_sequence() {
    let (directory, seals) = sealed_documents(Encoding::Json);
    let error = send_sealed(directory.path(), &seals, &mut Broken).unwrap_err();
    assert!(error.contains("stopped reading"), "{error}");
}
