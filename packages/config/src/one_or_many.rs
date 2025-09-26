//! A type that can represent either a single value or multiple values
//!
//! This provides an ergonomic way to handle APIs that can accept either
//! a single item or a collection of items without requiring separate code paths.

use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::SliceIndex;

/// A container that holds either one or many values of type T
///
/// This type is useful when you want to accept either a single value or
/// multiple values without forcing the caller to wrap single values in a Vec.
///
/// # Examples
///
/// ```rust
/// use progresshub_config::OneOrMany;
///
/// // From a single value
/// let single: OneOrMany<&str> = OneOrMany::from("hello");
/// assert_eq!(single.len(), 1);
///
/// // From multiple values
/// let multiple: OneOrMany<&str> = OneOrMany::from(vec!["hello", "world"]);
/// assert_eq!(multiple.len(), 2);
///
/// // Iterate over values
/// for value in &single {
///     println!("{}", value);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OneOrMany<T> {
    /// A single value
    One(T),
    /// Multiple values (must contain at least one)
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    /// Creates a new OneOrMany with a single value
    pub fn one(value: T) -> Self {
        OneOrMany::One(value)
    }

    /// Creates a new OneOrMany with multiple values
    ///
    /// # Panics
    /// Panics if the vector is empty
    pub fn many(values: Vec<T>) -> Self {
        assert!(!values.is_empty(), "OneOrMany::Many cannot be empty");
        OneOrMany::Many(values)
    }

    /// Returns the number of elements
    pub fn len(&self) -> usize {
        match self {
            OneOrMany::One(_) => 1,
            OneOrMany::Many(v) => v.len(),
        }
    }

    /// Returns true if this contains exactly one element
    pub fn is_one(&self) -> bool {
        matches!(self, OneOrMany::One(_))
    }

    /// Returns true if this contains multiple elements
    pub fn is_many(&self) -> bool {
        matches!(self, OneOrMany::Many(_))
    }

    /// Always returns false as OneOrMany cannot be empty
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Converts self into a Vec
    pub fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::One(v) => vec![v],
            OneOrMany::Many(v) => v,
        }
    }

    /// Returns an iterator over the elements
    pub fn iter(&self) -> Iter<'_, T> {
        match self {
            OneOrMany::One(v) => Iter::One(Some(v)),
            OneOrMany::Many(v) => Iter::Many(v.iter()),
        }
    }

    /// Returns a mutable iterator over the elements
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        match self {
            OneOrMany::One(v) => IterMut::One(Some(v)),
            OneOrMany::Many(v) => IterMut::Many(v.iter_mut()),
        }
    }

    /// Returns a slice of the elements
    pub fn as_slice(&self) -> &[T] {
        match self {
            OneOrMany::One(v) => std::slice::from_ref(v),
            OneOrMany::Many(v) => v.as_slice(),
        }
    }

    /// Returns a mutable slice of the elements
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self {
            OneOrMany::One(v) => std::slice::from_mut(v),
            OneOrMany::Many(v) => v.as_mut_slice(),
        }
    }

    /// Pushes a value, converting One to Many if needed
    pub fn push(&mut self, value: T) {
        match self {
            OneOrMany::One(_) => {
                let old = std::mem::replace(self, OneOrMany::Many(Vec::new()));
                if let OneOrMany::One(v) = old {
                    *self = OneOrMany::Many(vec![v, value]);
                }
            }
            OneOrMany::Many(v) => v.push(value),
        }
    }

    /// Extends with an iterator of values
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut iter = iter.into_iter();
        if let Some(first) = iter.next() {
            self.push(first);
            for item in iter {
                self.push(item);
            }
        }
    }

    /// Gets a reference to an element by index
    pub fn get(&self, index: usize) -> Option<&T> {
        match self {
            OneOrMany::One(v) if index == 0 => Some(v),
            OneOrMany::One(_) => None,
            OneOrMany::Many(v) => v.get(index),
        }
    }

    /// Gets a mutable reference to an element by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        match self {
            OneOrMany::One(v) if index == 0 => Some(v),
            OneOrMany::One(_) => None,
            OneOrMany::Many(v) => v.get_mut(index),
        }
    }

    /// Returns a reference to the first element
    pub fn first(&self) -> &T {
        match self {
            OneOrMany::One(v) => v,
            OneOrMany::Many(v) => &v[0], // Safe because Many is never empty
        }
    }

    /// Returns a mutable reference to the first element
    pub fn first_mut(&mut self) -> &mut T {
        match self {
            OneOrMany::One(v) => v,
            OneOrMany::Many(v) => &mut v[0],
        }
    }

    /// Returns a reference to the last element
    pub fn last(&self) -> &T {
        match self {
            OneOrMany::One(v) => v,
            OneOrMany::Many(v) => &v[v.len() - 1], // Safe: Many is guaranteed non-empty
        }
    }

    /// Returns a mutable reference to the last element
    pub fn last_mut(&mut self) -> &mut T {
        match self {
            OneOrMany::One(v) => v,
            OneOrMany::Many(v) => {
                let len = v.len();
                &mut v[len - 1] // Safe: Many is guaranteed non-empty
            }
        }
    }
}

// Conversion traits

impl<T> From<T> for OneOrMany<T> {
    fn from(value: T) -> Self {
        OneOrMany::One(value)
    }
}

impl<T> From<Vec<T>> for OneOrMany<T> {
    fn from(mut values: Vec<T>) -> Self {
        if values.is_empty() {
            panic!("Cannot create OneOrMany from empty Vec");
        } else if values.len() == 1 {
            // Safe: we've verified length is exactly 1, remove the single element
            OneOrMany::One(values.remove(0))
        } else {
            OneOrMany::Many(values)
        }
    }
}

impl<T, const N: usize> From<[T; N]> for OneOrMany<T> {
    fn from(arr: [T; N]) -> Self {
        assert!(N > 0, "Cannot create OneOrMany from empty array");
        let mut vec: Vec<T> = arr.into_iter().collect();
        if vec.len() == 1 {
            // Safe: we've verified length is exactly 1, remove the single element
            OneOrMany::One(vec.remove(0))
        } else {
            OneOrMany::Many(vec)
        }
    }
}

// Iterator support

/// Iterator over OneOrMany elements
pub enum Iter<'a, T> {
    One(Option<&'a T>),
    Many(std::slice::Iter<'a, T>),
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Iter::One(v) => v.take(),
            Iter::Many(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Iter::One(Some(_)) => (1, Some(1)),
            Iter::One(None) => (0, Some(0)),
            Iter::Many(iter) => iter.size_hint(),
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

/// Mutable iterator over OneOrMany elements
pub enum IterMut<'a, T> {
    One(Option<&'a mut T>),
    Many(std::slice::IterMut<'a, T>),
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IterMut::One(v) => v.take(),
            IterMut::Many(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            IterMut::One(Some(_)) => (1, Some(1)),
            IterMut::One(None) => (0, Some(0)),
            IterMut::Many(iter) => iter.size_hint(),
        }
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {}

// IntoIterator implementations

impl<T> IntoIterator for OneOrMany<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            OneOrMany::One(v) => IntoIter::One(Some(v)),
            OneOrMany::Many(v) => IntoIter::Many(v.into_iter()),
        }
    }
}

impl<'a, T> IntoIterator for &'a OneOrMany<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut OneOrMany<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Owned iterator over OneOrMany elements
pub enum IntoIter<T> {
    One(Option<T>),
    Many(std::vec::IntoIter<T>),
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IntoIter::One(v) => v.take(),
            IntoIter::Many(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            IntoIter::One(Some(_)) => (1, Some(1)),
            IntoIter::One(None) => (0, Some(0)),
            IntoIter::Many(iter) => iter.size_hint(),
        }
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

// Index implementations

impl<T, I> Index<I> for OneOrMany<T>
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<T, I> IndexMut<I> for OneOrMany<T>
where
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_mut_slice().index_mut(index)
    }
}

// Deref to slice for convenience

impl<T> Deref for OneOrMany<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for OneOrMany<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

// Default implementation

impl<T> Default for OneOrMany<T>
where
    T: Default,
{
    fn default() -> Self {
        OneOrMany::One(T::default())
    }
}
