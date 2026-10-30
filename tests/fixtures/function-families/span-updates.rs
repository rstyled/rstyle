// Appended to position-span.rs for Cargo capture; separate development selection.
impl Span {
    pub fn with_start(self, pos: Position) -> Span { Span { start: pos, ..self } }
    pub fn with_end(self, pos: Position) -> Span { Span { end: pos, ..self } }
    pub fn mutable_self(mut self, pos: Position) -> Span { self.start = pos; self }
    pub fn typed_self(self: Self, pos: Position) -> Span { Span { start: pos, ..self } }
}
pub fn updated_both(span: Span, start: Position, end: Position) -> Span {
    span.with_start(start).with_end(end)
}
pub fn updated_empty(span: Span, pos: Position) -> bool { span.with_start(pos).is_empty() }
pub fn result_update(start: Position, end: Position, pos: Position) -> Span {
    Span::new(start, end).with_end(pos)
}
pub fn ufcs_update(span: Span, pos: Position) -> Span { Span::with_end(span, pos) }
pub fn retained(span: Span, pos: Position) -> (Span, Span) { (span, span.with_start(pos)) }
pub fn shared_to_owned(span: &Span, pos: Position) -> Span { span.with_start(pos) }
