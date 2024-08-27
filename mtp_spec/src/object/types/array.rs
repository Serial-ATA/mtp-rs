use core::ops::{Index, IndexMut};
use core::slice::Iter;

/// A fixed-size array of elements of type `T`.
///
/// MTP defines as a fixed-size concatenation of **integer** elements.
///
/// See [`ArrayEncodable`] for a list of types that can be used in an `Array`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Array<T: ArrayEncodable>(Box<[T]>);

impl<T> FromIterator<T> for Array<T>
where
	T: ArrayEncodable,
{
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = T>,
	{
		Self(iter.into_iter().collect())
	}
}

impl<T> Index<usize> for Array<T>
where
	T: ArrayEncodable,
{
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		&self.0[index]
	}
}

impl<T> IndexMut<usize> for Array<T>
where
	T: ArrayEncodable,
{
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		&mut self.0[index]
	}
}

impl<T> From<Vec<T>> for Array<T>
where
	T: ArrayEncodable,
{
	fn from(value: Vec<T>) -> Self {
		Self(value.into_boxed_slice())
	}
}

impl<T: ArrayEncodable> Array<T> {
	/// Get the length of the array.
	///
	/// # Examples
	///
	/// ```
	/// use mtp_spec::object::types::Array;
	///
	/// let array = Array::from(vec![1, 2, 3]);
	/// assert_eq!(array.len(), 3);
	/// ```
	pub fn len(&self) -> usize {
		self.0.len()
	}

	/// Returns `true` if the array is empty.
	///
	/// # Examples
	///
	/// ```
	/// use mtp_spec::object::types::Array;
	///
	/// let array = Array::from(vec![1, 2, 3]);
	/// assert!(!array.is_empty());
	///
	/// let empty_array = Array::<u8>::from(vec![]);
	/// assert!(empty_array.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Returns an iterator over the elements of the array.
	///
	/// # Examples
	///
	/// ```
	/// use mtp_spec::object::types::Array;
	///
	/// let array = Array::from(vec![1, 2, 3]);
	/// let mut iter = array.iter();
	///
	/// assert_eq!(iter.next(), Some(&1));
	/// assert_eq!(iter.next(), Some(&2));
	/// assert_eq!(iter.next(), Some(&3));
	/// assert_eq!(iter.next(), None);
	/// ```
	pub fn iter(&self) -> Iter<'_, T> {
		self.0.iter()
	}
}

/// Marker trait for types that can be encoded in an [`Array`].
pub trait ArrayEncodable {}

impl ArrayEncodable for u8 {}
impl ArrayEncodable for u16 {}
impl ArrayEncodable for u32 {}
impl ArrayEncodable for u64 {}
impl ArrayEncodable for u128 {}
impl ArrayEncodable for i8 {}
impl ArrayEncodable for i16 {}
impl ArrayEncodable for i32 {}
impl ArrayEncodable for i64 {}
impl ArrayEncodable for i128 {}
