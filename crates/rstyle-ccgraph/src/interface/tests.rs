//! Differential checks against the original exhaustive greedy window scan.
use super::*;

fn scanned(short: &[char], long: &[char]) -> MatchState {
    let radius = short_long_window(short.len(), long.len());
    let mut used = vec![false; long.len()];
    let mut selected = vec![false; short.len()];
    for (i, character) in short.iter().enumerate() {
        let lo = i.saturating_sub(radius);
        let hi = (i + radius).min(long.len().saturating_sub(1));
        for j in lo..=hi {
            if !used[j] && long[j] == *character {
                used[j] = true;
                selected[i] = true;
                break;
            }
        }
    }
    let ordered_short = picked(short, &selected);
    MatchState {
        count: ordered_short.len(),
        ordered_short,
        ordered_long: picked(long, &used),
    }
}

fn compare(first: &[char], second: &[char]) {
    let (short, long) = orient(first, second);
    let expected = scanned(short, long);
    let actual = find_matches(short, long);
    assert_eq!(actual.count, expected.count);
    assert_eq!(actual.ordered_short, expected.ordered_short);
    assert_eq!(actual.ordered_long, expected.ordered_long);
}

#[test]
fn indexed_matching_preserves_every_greedy_choice_on_small_unicode_strings() {
    let mut words = vec![Vec::new()];
    let mut level = vec![Vec::new()];
    for _ in 0..4 {
        level = level
            .iter()
            .flat_map(|prefix| {
                ['a', 'b', '🦀'].map(|character| {
                    let mut word = prefix.clone();
                    word.push(character);
                    word
                })
            })
            .collect();
        words.extend(level.iter().cloned());
    }
    for first in &words {
        for second in &words {
            if !first.is_empty() && !second.is_empty() {
                compare(first, second);
            }
        }
    }
}

#[test]
fn indexed_matching_preserves_expired_unmatched_positions_and_long_repeated_runs() {
    for (first, second) in [
        ("abbbbbb", "bbbbbab"),
        ("abcdef", "zyxwvu"),
        ("ab", "ba"),
        ("é🦀aé🦀", "🦀é🦀aé"),
    ] {
        compare(
            &first.chars().collect::<Vec<_>>(),
            &second.chars().collect::<Vec<_>>(),
        );
    }
    let first: Vec<_> = format!("{}b{}", "a".repeat(2048), "a".repeat(2048))
        .chars()
        .collect();
    let second: Vec<_> = format!("{}c{}", "a".repeat(2048), "a".repeat(2048))
        .chars()
        .collect();
    compare(&first, &second);
}
