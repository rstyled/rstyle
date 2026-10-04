//! Left spellings: `while` counter, `match`, `if/else if` chain, `&&` condition.
use crate::model::{Registry, Sink};

// S1: retry three times (while + manual counter)
pub fn retry_while(sink: &mut dyn Sink, msg: &str) -> Result<(), String> {
    let mut tries = 0;
    while tries < 3 {
        tries += 1;
        if sink.put(msg).is_ok() {
            return Ok(());
        }
        sink.note(&format!("retry #{tries} for {msg}"));
    }
    Err(format!("gave up on {msg}"))
}

// S2: hit / miss (match)
pub fn lookup_match(reg: &Registry, key: &str) -> String {
    match reg.find(key) {
        Some(v) => {
            reg.touch(key);
            format!("hit {key}={v}")
        }
        None => {
            reg.miss(key);
            format!("miss {key}")
        }
    }
}

// S3: status classification (if / else if chain)
pub fn react_chain(reg: &Registry, key: &str, code: u16) -> String {
    if code == 200 {
        reg.touch(key);
        format!("ok {key}")
    } else if code == 404 {
        reg.miss(key);
        format!("missing {key}")
    } else {
        reg.note(key);
        format!("error {key} ({code})")
    }
}

// S4: cached read (&& condition)
pub fn cached_and(reg: &Registry, key: &str) -> Option<String> {
    if reg.has(key) && reg.fresh(key) {
        reg.touch(key);
        Some(reg.read(key))
    } else {
        reg.miss(key);
        None
    }
}

// D1: looks like S1 (while + counter + sink calls) but collects and never fails
pub fn flush_all(sink: &mut dyn Sink, lines: &[String]) -> usize {
    let mut sent = 0;
    while sent < lines.len() {
        sink.note(&lines[sent]);
        sent += 1;
    }
    sink.note(&format!("flushed {sent}"));
    sent
}

// S7: write, read back, write again (`?`)
pub fn store_try(reg: &Registry, sink: &mut dyn Sink, key: &str) -> Result<String, String> {
    sink.put(key)?;
    let value = reg.read(key);
    sink.put(&value)?;
    Ok(value)
}
