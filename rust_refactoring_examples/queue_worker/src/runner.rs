pub trait Handler {
    fn kind(&self) -> &'static str;
    fn handle(&self, payload: &str, attempt: u32) -> Result<String, String>;
}

#[derive(Default)]
pub struct Registry {
    handlers: Vec<Box<dyn Handler>>,
}

impl Registry {
    pub fn add(&mut self, handler: Box<dyn Handler>) {
        self.handlers.push(handler);
    }

    pub fn run(&self, kind: &str, payload: &str, attempt: u32) -> Result<String, String> {
        let handler = self
            .handlers
            .iter()
            .find(|handler| handler.kind() == kind)
            .ok_or_else(|| format!("no handler for {kind}"))?;
        handler.handle(payload, attempt)
    }
}
