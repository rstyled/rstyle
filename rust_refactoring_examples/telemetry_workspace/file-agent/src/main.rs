use telemetry_core::{event, EventSink, VecSink};

fn clean_line(line: &str) -> Option<String> {
    let value = line.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn file_timestamp(sequence: u64) -> u64 {
    1_700_000_000 + sequence
}

fn persist_with_retries(
    sink: &mut impl EventSink,
    event: &telemetry_core::Event,
) -> Result<(), String> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        match sink.write(event) {
            Ok(()) => return Ok(()),
            Err(err) if attempts < 3 => eprintln!("file write retry {attempts}: {err}"),
            Err(err) => return Err(format!("file ingestion failed after {attempts} attempts: {err}")),
        }
    }
}

fn ingest_lines(lines: &[&str], sink: &mut impl EventSink) -> Result<(), String> {
    for (index, line) in lines.iter().enumerate() {
        let Some(payload) = clean_line(line) else {
            continue;
        };
        let event = event!("file", payload, file_timestamp(index as u64));
        persist_with_retries(sink, &event)?;
    }
    Ok(())
}

fn main() {
    let mut sink = VecSink {
        fail_next: true,
        ..VecSink::default()
    };

    ingest_lines(&[" cpu=0.42 ", "", "mem=812"], &mut sink).unwrap();
    println!("file agent stored {} events", sink.events.len());
}
