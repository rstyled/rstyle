//! Native oracle against unchanged num-traits; no extraction/admission claim.
use std::cell::RefCell;
use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

#[derive(Default)]
struct Counts {
    scalar_calls: usize,
    expected_panics: usize,
    custom_controls: usize,
}

macro_rules! scalar_domain {
    ($name:ident, $ty:ty) => {
        fn $name(triples: &[$ty], pairs: &[$ty], counts: &mut Counts) {
            for &input in triples {
                for &min in triples {
                    for &max in triples {
                        // Independent piecewise oracle, including invalid bounds.
                        if cfg!(debug_assertions) && min > max {
                            assert!(catch_unwind(|| num_traits::clamp(input, min, max)).is_err());
                            counts.expected_panics += 1;
                        } else {
                            let expected = if min <= max {
                                input.max(min).min(max)
                            } else if input < min {
                                min
                            } else {
                                max
                            };
                            assert_eq!(num_traits::clamp(input, min, max), expected);
                        }
                        counts.scalar_calls += 1;
                    }
                }
            }
            for &input in pairs {
                for &bound in pairs {
                    assert_eq!(num_traits::clamp_min(input, bound), input.max(bound));
                    assert_eq!(num_traits::clamp_max(input, bound), input.min(bound));
                    counts.scalar_calls += 2;
                }
            }
        }
    };
}

scalar_domain!(signed, i32);
scalar_domain!(unsigned, u32);

struct Effectful {
    value: i32,
    events: Rc<RefCell<Vec<String>>>,
}

impl PartialEq for Effectful {
    fn eq(&self, other: &Self) -> bool {
        self.events
            .borrow_mut()
            .push(format!("eq:{}:{}", self.value, other.value));
        self.value == other.value
    }
}

impl PartialOrd for Effectful {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.events
            .borrow_mut()
            .push(format!("cmp:{}:{}", self.value, other.value));
        self.value.partial_cmp(&other.value)
    }
}

fn effectful_controls(counts: &mut Counts) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let make = |value| Effectful {
        value,
        events: Rc::clone(&events),
    };
    assert_eq!(num_traits::clamp(make(5), make(0), make(10)).value, 5);
    let expected = if cfg!(debug_assertions) {
        vec!["cmp:0:10", "cmp:5:0", "cmp:5:10"]
    } else {
        vec!["cmp:5:0", "cmp:5:10"]
    };
    assert_eq!(*events.borrow(), expected);
    events.borrow_mut().clear();
    assert_eq!(num_traits::clamp_min(make(5), make(0)).value, 5);
    assert_eq!(
        *events.borrow(),
        if cfg!(debug_assertions) {
            vec!["eq:0:0", "cmp:5:0"]
        } else {
            vec!["cmp:5:0"]
        }
    );
    events.borrow_mut().clear();
    assert_eq!(num_traits::clamp_max(make(5), make(10)).value, 5);
    assert_eq!(
        *events.borrow(),
        if cfg!(debug_assertions) {
            vec!["eq:10:10", "cmp:5:10"]
        } else {
            vec!["cmp:5:10"]
        }
    );
    counts.custom_controls += 3;
}

struct NonReflexive(i32);

impl PartialEq for NonReflexive {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl PartialOrd for NonReflexive {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
}

fn nonreflexive_controls(counts: &mut Counts) {
    for operation in 0..3 {
        let result = catch_unwind(AssertUnwindSafe(|| match operation {
            0 => num_traits::clamp(NonReflexive(5), NonReflexive(0), NonReflexive(10)),
            1 => num_traits::clamp_min(NonReflexive(5), NonReflexive(0)),
            _ => num_traits::clamp_max(NonReflexive(5), NonReflexive(10)),
        }));
        if cfg!(debug_assertions) {
            assert!(result.is_err());
            counts.expected_panics += 1;
        } else {
            assert_eq!(result.unwrap_or_else(|_| panic!("unexpected panic")).0, 5);
        }
        counts.custom_controls += 1;
    }
}

fn main() {
    // Expected caught panics are counted rather than printed thousands of times.
    std::panic::set_hook(Box::new(|_| {}));
    let mut counts = Counts::default();
    let signed_triples: Vec<i32> = (-8..=8).collect();
    let unsigned_triples: Vec<u32> = (0..=16).collect();
    let signed_pairs: Vec<i32> = (-128..=127).collect();
    let unsigned_pairs: Vec<u32> = (0..=255).collect();
    signed(&signed_triples, &signed_pairs, &mut counts);
    unsigned(&unsigned_triples, &unsigned_pairs, &mut counts);
    let signed_edges = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let unsigned_edges = [0, 1, 127, 255, 65535, u32::MAX - 1, u32::MAX];
    signed(&signed_edges, &signed_edges, &mut counts);
    unsigned(&unsigned_edges, &unsigned_edges, &mut counts);
    effectful_controls(&mut counts);
    nonreflexive_controls(&mut counts);
    println!(
        "{{\"debug_assertions\":{},\"scalar_calls\":{},\"expected_panics\":{},\"custom_unsupported_controls\":{},\"mismatches\":0}}",
        cfg!(debug_assertions),
        counts.scalar_calls,
        counts.expected_panics,
        counts.custom_controls
    );
}
