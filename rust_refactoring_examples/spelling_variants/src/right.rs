//! Right spellings: `for` range, `if let`, `match` on the code, nested `if`.
use crate::model::{Registry, Sink};

// S1: retry three times (for over a range)
pub fn retry_for(sink: &mut dyn Sink, msg: &str) -> Result<(), String> {
    for attempt in 1..=3 {
        if sink.put(msg).is_ok() {
            return Ok(());
        }
        sink.note(&format!("retry #{attempt} for {msg}"));
    }
    Err(format!("gave up on {msg}"))
}

// S2: hit / miss (if let)
pub fn lookup_if_let(reg: &Registry, key: &str) -> String {
    if let Some(v) = reg.find(key) {
        reg.touch(key);
        format!("hit {key}={v}")
    } else {
        reg.miss(key);
        format!("miss {key}")
    }
}

// S3: status classification (match)
pub fn react_match(reg: &Registry, key: &str, code: u16) -> String {
    match code {
        200 => {
            reg.touch(key);
            format!("ok {key}")
        }
        404 => {
            reg.miss(key);
            format!("missing {key}")
        }
        _ => {
            reg.note(key);
            format!("error {key} ({code})")
        }
    }
}

// S4: cached read (nested if)
pub fn cached_nested(reg: &Registry, key: &str) -> Option<String> {
    if reg.has(key) {
        if reg.fresh(key) {
            reg.touch(key);
            return Some(reg.read(key));
        }
    }
    reg.miss(key);
    None
}

// D2: looks like S2 (two-arm lookup, hit / miss strings) but evicts instead of touching
pub fn evict_if_let(reg: &mut Registry, key: &str) -> String {
    if let Some(v) = reg.items.remove(key) {
        reg.note(&format!("evicted {key}"));
        format!("hit {key}={v}")
    } else {
        reg.fresh_keys.retain(|k| k != key);
        format!("miss {key}")
    }
}

// S7: write, read back, write again (explicit `match` + early return)
pub fn store_match(reg: &Registry, sink: &mut dyn Sink, key: &str) -> Result<String, String> {
    match sink.put(key) {
        Ok(()) => {}
        Err(err) => return Err(err),
    }
    let value = reg.read(key);
    match sink.put(&value) {
        Ok(()) => {}
        Err(err) => return Err(err),
    }
    Ok(value)
}
