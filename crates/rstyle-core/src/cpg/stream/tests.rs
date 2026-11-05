use super::*;
use std::cell::Cell;

fn example() -> Value {
    serde_json::from_str(include_str!("../testdata/example.cpg.json")).unwrap()
}

#[test]
fn bundled_records_reach_the_visitor_before_the_following_document_is_read() {
    let visited = Cell::new(0);
    let documents = (0..3).map(|i| {
        assert_eq!(visited.get(), i * 3);
        let mut value = example();
        value["unit"]["id"] = Value::String(format!("unit:{i}"));
        Ok(value)
    });
    visit_records(documents, Demand::default(), |batch| {
        visited.set(visited.get() + batch.len());
        Ok(())
    })
    .unwrap();
    assert_eq!(visited.get(), 9);
}

#[test]
fn split_functions_can_arrive_before_their_unit() {
    let mut split = layout::split(&example());
    split.reverse();
    let records = read_records(
        split.into_iter().map(Ok),
        Demand {
            graphs: true,
            bounds: true,
        },
    )
    .unwrap();
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|record| record.bounds.is_some()));
}

#[test]
fn stream_rejects_duplicate_functions_across_bundled_and_split_documents() {
    let documents = std::iter::once(example())
        .chain(layout::split(&example()))
        .map(Ok);
    let error = read_records(documents, Demand::default()).unwrap_err();
    assert!(error.contains("appears more than once"), "{error}");
}

#[test]
fn late_input_failure_does_not_produce_a_successful_run_or_read_more_input() {
    let visited = Cell::new(0);
    let documents = [
        Ok(example()),
        Err("incomplete transfer".into()),
        Ok(example()),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, value)| {
        assert!(i < 2, "read after input failure");
        value
    });
    let error = visit_records(documents, Demand::default(), |batch| {
        visited.set(visited.get() + batch.len());
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error, "incomplete transfer");
    assert_eq!(visited.get(), 3);
}
