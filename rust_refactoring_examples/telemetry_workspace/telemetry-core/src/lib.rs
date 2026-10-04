use std::fmt;

#[derive(Clone, Debug)]
pub struct Event {
    pub source: String,
    pub payload: String,
    pub observed_at: u64,
}

pub trait EventSink {
    fn write(&mut self, event: &Event) -> Result<(), SinkError>;
}

#[derive(Debug)]
pub struct SinkError(pub &'static str);

impl fmt::Display for SinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[macro_export]
macro_rules! event {
    ($source:expr, $payload:expr, $observed_at:expr) => {
        $crate::Event {
            source: $source.to_string(),
            payload: $payload.to_string(),
            observed_at: $observed_at,
        }
    };
}

#[derive(Default)]
pub struct VecSink {
    pub events: Vec<Event>,
    pub fail_next: bool,
}

impl EventSink for VecSink {
    fn write(&mut self, event: &Event) -> Result<(), SinkError> {
        if self.fail_next {
            self.fail_next = false;
            return Err(SinkError("temporary sink failure"));
        }
        self.events.push(event.clone());
        Ok(())
    }
}
