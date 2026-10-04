use telemetry_core::{event, EventSink, VecSink};

fn normalize_body(body: &str) -> Result<String, &'static str> {
    let normalized = body.trim();
    if normalized.is_empty() {
        Err("empty request body")
    } else {
        Ok(normalized.to_string())
    }
}

fn received_at(request_number: u64) -> u64 {
    1_700_100_000 + request_number
}

fn send_with_retry(
    sink: &mut impl EventSink,
    event: &telemetry_core::Event,
) -> Result<(), String> {
    for attempt in 1..=3 {
        if let Err(err) = sink.write(event) {
            if attempt == 3 {
                return Err(format!("http ingestion gave up: {err}"));
            }
            eprintln!("http sink retry #{attempt}: {err}");
        } else {
            return Ok(());
        }
    }
    unreachable!()
}

fn handle_request(number: u64, body: &str, sink: &mut impl EventSink) -> Result<(), String> {
    let payload = normalize_body(body).map_err(str::to_string)?;
    let event = event!("http", payload, received_at(number));
    send_with_retry(sink, &event)
}

fn main() {
    let mut sink = VecSink::default();
    handle_request(17, "latency_ms=23", &mut sink).unwrap();
    println!("http agent stored {} events", sink.events.len());
}
