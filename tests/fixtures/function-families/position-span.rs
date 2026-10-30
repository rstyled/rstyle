// Independently specified development composition, separate from frozen source.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Position { pub offset: usize, pub line: usize, pub column: usize }
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Span { pub start: Position, pub end: Position }
impl Position {
    pub fn new(offset: usize, line: usize, column: usize) -> Position {
        Position { offset, line, column }
    }
}
impl Span {
    pub fn new(start: Position, end: Position) -> Span { Span { start, end } }
    pub fn is_empty(&self) -> bool { self.start.offset == self.end.offset }
    pub fn is_one_line(&self) -> bool { self.start.line == self.end.line }
    pub fn splat(position: Position) -> Span { Span::new(position, position) }
}
pub fn composed(x: usize, y: usize) -> bool {
    let start = Position::new(x, 1, 2);
    let end = Position::new(y, 1, 3);
    let span = Span::new(start, end);
    span.is_empty()
}
pub fn same_empty(span: &Span) -> bool { span.start.offset == span.end.offset }
pub fn different_empty(span: &Span) -> bool { span.start.column == span.end.column }
pub fn constructed_fields(x: usize, y: usize) -> bool {
    let start = Position::new(x, 1, 2);
    let end = Position::new(y, 1, 3);
    let span = Span::new(start, end);
    span.start.offset == span.end.offset
}
pub fn shared_getter(span: &Span) -> bool { span.is_empty() }
