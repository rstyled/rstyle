//! Uncompressed RFC 7464 object records, committed by a terminal null and EOF.
use super::Encoding;
use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, BufRead, BufReader, Read};

const RS: u8 = 0x1e;

/// Observations of consumed input, without a manifest or independently verified seals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SequenceProgress {
    pub framing: &'static str,
    pub encoding: Encoding,
    pub received_documents: usize,
    pub received_bytes: u64,
    pub complete: bool,
}

#[derive(PartialEq, Eq)]
enum State {
    Open,
    Complete,
    Failed,
}

impl State {
    fn after<T>(result: &Result<Option<T>, String>) -> Self {
        match result {
            Ok(Some(_)) => Self::Open,
            Ok(None) => Self::Complete,
            Err(_) => Self::Failed,
        }
    }
}

/// Reads one object at a time without a whole-document byte buffer or disk staging.
/// Earlier documents remain provisional until `complete()` succeeds. CPG validation
/// belongs to the caller, just as it does for `CaptureReader`.
pub struct SequenceReader<R> {
    input: Metered<R>,
    documents: usize,
    state: State,
}

impl<R: BufRead> SequenceReader<R> {
    pub fn new(input: R) -> Self {
        Self {
            input: Metered {
                inner: input,
                bytes: 0,
            },
            documents: 0,
            state: State::Open,
        }
    }

    pub fn progress(&self) -> SequenceProgress {
        SequenceProgress {
            framing: "rfc7464",
            encoding: Encoding::Json,
            received_documents: self.documents,
            received_bytes: self.input.bytes,
            complete: self.state == State::Complete,
        }
    }

    /// The callback must consume and validate the entire record. Storage helpers
    /// can copy directly into an encoder; analysis callers can deserialize directly.
    pub fn next_with<T>(
        &mut self,
        read: impl FnOnce(&mut dyn Read) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        if self.state != State::Open {
            return Ok(None);
        }
        self.advance(read)
    }

    fn advance<T>(
        &mut self,
        read: impl FnOnce(&mut dyn Read) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        let result = self.read_next(read);
        self.state = State::after(&result);
        result
    }

    pub fn next_document<T: DeserializeOwned>(&mut self) -> Result<Option<T>, String> {
        self.next_with(|record| {
            serde_json::from_reader(BufReader::new(super::read::Utf8::new(record)))
                .map_err(|error| error.to_string())
        })
    }

    fn read_next<T>(
        &mut self,
        read: impl FnOnce(&mut dyn Read) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        match start_record(&mut self.input)? {
            true => self.read_document(read).map(Some),
            false => self.finish().map(|()| None),
        }
    }

    fn read_document<T>(
        &mut self,
        read: impl FnOnce(&mut dyn Read) -> Result<T, String>,
    ) -> Result<T, String> {
        let value = document(&mut self.input, read)
            .map_err(|error| format!("CPG stream document {}: {error}", self.documents + 1))?;
        self.documents += 1;
        Ok(value)
    }

    fn finish(&mut self) -> Result<(), String> {
        terminal(&mut self.input)?;
        if self.documents == 0 {
            return Err("A capture stream must contain at least one document".into());
        }
        Ok(())
    }

    /// The next record's bytes, unparsed, so another thread can parse them.
    pub fn next_bytes(&mut self) -> Result<Option<Vec<u8>>, String> {
        self.next_with(|record| {
            let mut bytes = Vec::new();
            record
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            Ok(bytes)
        })
    }

    /// Success requires the terminal record and EOF, not merely a valid prefix.
    pub fn complete(&self) -> Result<(), String> {
        match self.state {
            State::Complete => Ok(()),
            _ => Err("CPG stream was not completely received".into()),
        }
    }
}

impl<R: BufRead> Iterator for SequenceReader<R> {
    type Item = Result<serde_json::Value, String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_document().transpose()
    }
}

fn start_record(input: &mut impl BufRead) -> Result<bool, String> {
    separator(input)?;
    is_document(
        input
            .fill_buf()
            .map_err(|error| error.to_string())?
            .first()
            .copied(),
    )
}

fn separator(input: &mut impl Read) -> Result<(), String> {
    let mut separator = [0];
    input.read_exact(&mut separator).map_err(separator_error)?;
    expect_separator(separator[0])
}

fn separator_error(error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::UnexpectedEof => {
            "Capture stream ended before its terminal null record".into()
        }
        _ => format!("Cannot read CPG stream: {error}"),
    }
}

fn expect_separator(byte: u8) -> Result<(), String> {
    if byte != RS {
        return Err("Expected a JSON text sequence record separator (0x1e)".into());
    }
    Ok(())
}

fn is_document(first: Option<u8>) -> Result<bool, String> {
    match first {
        Some(b'{') => Ok(true),
        Some(b'n') => Ok(false),
        _ => {
            Err("Each capture record must start with a JSON object; finish with RS null LF".into())
        }
    }
}

fn terminal(input: &mut impl BufRead) -> Result<(), String> {
    let mut null = [0; 4];
    input
        .read_exact(&mut null)
        .map_err(|error| error.to_string())?;
    expect_null(&null)?;
    trailing_whitespace(input)
}

fn expect_null(bytes: &[u8]) -> Result<(), String> {
    if bytes != b"null" {
        return Err("Expected a terminal null record".into());
    }
    Ok(())
}

fn trailing_whitespace(input: &mut impl BufRead) -> Result<(), String> {
    while whitespace_chunk(input)? {}
    Ok(())
}

fn whitespace_chunk(input: &mut impl BufRead) -> Result<bool, String> {
    let bytes = input.fill_buf().map_err(|error| error.to_string())?;
    let len = whitespace_length(bytes)?;
    input.consume(len);
    Ok(len > 0)
}

fn whitespace_length(bytes: &[u8]) -> Result<usize, String> {
    if !bytes
        .iter()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
    {
        return Err("The terminal null record must be followed only by whitespace and EOF".into());
    }
    Ok(bytes.len())
}

fn document<T>(
    input: &mut impl BufRead,
    read: impl FnOnce(&mut dyn Read) -> Result<T, String>,
) -> Result<T, String> {
    let mut record = Record { input };
    let value = read(&mut record)?;
    consumed(&mut record)?;
    Ok(value)
}

fn consumed(input: &mut impl Read) -> Result<(), String> {
    if input.read(&mut [0]).map_err(|error| error.to_string())? != 0 {
        return Err("The document reader did not consume its entire record".into());
    }
    Ok(())
}

/// Stops before the next RS, even if the underlying buffer holds several records.
struct Record<'a, R> {
    input: &'a mut R,
}

impl<R: BufRead> Read for Record<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        self.read_chunk(out)
    }
}

impl<R: BufRead> Record<'_, R> {
    fn read_chunk(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let bytes = self.input.fill_buf()?;
        let end = record_length(bytes, out.len());

        out[..end].copy_from_slice(&bytes[..end]);
        self.input.consume(end);
        Ok(end)
    }
}

fn record_length(bytes: &[u8], capacity: usize) -> usize {
    bytes
        .iter()
        .position(|byte| *byte == RS)
        .unwrap_or(bytes.len())
        .min(capacity)
}

struct Metered<R> {
    inner: R,
    bytes: u64,
}

impl<R: Read> Read for Metered<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let len = self.inner.read(out)?;
        self.bytes += len as u64;
        Ok(len)
    }
}

impl<R: BufRead> BufRead for Metered<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, len: usize) {
        self.inner.consume(len);
        self.bytes += len as u64;
    }
}

#[cfg(test)]
mod tests;
