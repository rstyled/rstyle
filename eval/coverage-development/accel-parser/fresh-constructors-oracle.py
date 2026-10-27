#!/usr/bin/env python3
"""Native checks of existing development constructors; not natural-crate evidence."""
import json
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parent.parent
scalar = (root / 'borrowed-scalar-owners.rs').read_text()
accel = (root / 'borrowed-accel-views.rs').read_text()
position = (root / 'position-span.rs').read_text()
assert position.count('Clone, Copy, Eq, PartialEq') == 2
position = position.replace('Clone, Copy, Eq, PartialEq', 'Clone, Eq, PartialEq').replace(
    'Span::new(position, position)', 'Span::new(position.clone(), position)')
source = ('mod scalar {\n' + scalar + '\n}\nmod position {\n' + position
          + '\n}\nmod accel {\n' + accel + '''
pub fn probe(input: [u8; 4]) -> [u8; 8] { Accel::from_bytes_unchecked(input).bytes }
}
fn check(input: [u8; 4]) {
    let actual = accel::probe(input);
    assert_eq!((actual[0], actual[1], actual[2], actual[3]),
               (input[0], input[1], input[2], input[3]));
    assert_eq!(&actual[4..], &[0, 0, 0, 0]);
}
fn main() {
    for (start, end) in [(0, 0), (0, u32::MAX), (u32::MAX, 0),
                         (u32::MAX, u32::MAX), (17, 31)] {
        let result = scalar::ScalarRange::owned(start, end);
        assert_eq!((result.start, result.end), (start, end));
    }
    for (offset, line, column) in [(3, 5, 7), (usize::MAX, 0, 1), (0, usize::MAX, 0)] {
        let result = position::Position::new(offset, line, column);
        assert_eq!((result.offset, result.line, result.column), (offset, line, column));
    }
    for first in 0..=255 {
        check([first, 17, 31, 251]);
        check([first, 0, 255, 165]);
    }
    for position in 1..4 {
        for value in 0..=255 {
            for first in [0, 255] {
                let mut input = [first, 17, 31, 251];
                input[position] = value;
                check(input);
            }
        }
    }
}
''')
with tempfile.TemporaryDirectory(prefix='rstyle-fresh-constructors-') as temporary:
    directory = Path(temporary)
    (directory / 'main.rs').write_text(source)
    subprocess.run([str(Path.home() / '.cargo/bin/rustc'), '--edition=2021',
                    '-C', 'opt-level=0', '-C', 'overflow-checks=yes',
                    str(directory / 'main.rs'), '-o', str(directory / 'oracle')],
                   check=True, capture_output=True, timeout=60)
    subprocess.run([str(directory / 'oracle')], check=True, timeout=30)
print(json.dumps(dict(development_only=True, scalar_cases=5, position_cases=3,
                      unchecked_array_cases=2048, mismatches=0)))
