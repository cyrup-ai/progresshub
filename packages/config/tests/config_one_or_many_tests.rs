//! Tests for OneOrMany type functionality
//!
//! This module contains comprehensive tests for the OneOrMany configuration type
//! including single item, multiple items, iteration, and edge case handling.
//! Extracted from src/one_or_many.rs for production code organization.

use progresshub_config::OneOrMany;

#[test]
fn test_one() {
    let one = OneOrMany::from(42);
    assert_eq!(one.len(), 1);
    assert!(one.is_one());
    assert!(!one.is_many());
    assert_eq!(one.first(), &42);
    assert_eq!(one.last(), &42);
}

#[test]
fn test_many() {
    let many: OneOrMany<i32> = OneOrMany::from(vec![1, 2, 3]);
    assert_eq!(many.len(), 3);
    assert!(!many.is_one());
    assert!(many.is_many());
    assert_eq!(many.first(), &1);
    assert_eq!(many.last(), &3);
}

#[test]
fn test_iter() {
    let one = OneOrMany::from(42);
    let vec: Vec<_> = one.iter().copied().collect();
    assert_eq!(vec, vec![42]);

    let many: OneOrMany<i32> = OneOrMany::from(vec![1, 2, 3]);
    let vec: Vec<_> = many.iter().copied().collect();
    assert_eq!(vec, vec![1, 2, 3]);
}

#[test]
fn test_push() {
    let mut one = OneOrMany::from(1);
    one.push(2);
    assert!(one.is_many());
    assert_eq!(one.len(), 2);
    assert_eq!(one.as_slice(), &[1, 2]);
}

#[test]
#[should_panic(expected = "Cannot create OneOrMany from empty Vec")]
fn test_empty_vec_panics() {
    let _: OneOrMany<i32> = Vec::new().into();
}
