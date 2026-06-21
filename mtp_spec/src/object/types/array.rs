use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::ops::{Index, IndexMut};
use core::slice::Iter;

use deku::ctx::Endian;
use deku::{DekuReader, DekuWriter, deku_derive};

/// A fixed-size array of elements of type `T`.
///
/// MTP defines as a fixed-size concatenation of **integer** elements.
///
/// See [`ArrayEncodable`] for a list of types that can be used in an `Array`.
#[deku_derive(DekuRead, DekuWrite)]
#[derive(Clone, Eq, PartialEq)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct Array<T: ArrayEncodable>(
    // Array Definition
    //
    // | Field                     | Size (bytes) | Format  |
    // |---------------------------|--------------|---------|
    // | NumElements               | 4            | UINT32  |
    // | ArrayEntry[0]             | Element Size | Special |
    // | ArrayEntry[1]             | Element Size | Special |
    // | ...                       | ...          | ...     |
    // | ArrayEntry[NumElements-1] | Element Size | Special |
    #[deku(temp, temp_value = "field_1.len() as u32")] u32,
    #[deku(count = "field_0")] Box<[T]>,
);

impl<T> Debug for Array<T>
where
    T: ArrayEncodable + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> FromIterator<T> for Array<T>
where
    T: ArrayEncodable,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self(iter.into_iter().take(u32::MAX as usize).collect())
    }
}

impl<T> IntoIterator for Array<T>
where
    T: ArrayEncodable,
{
    type Item = T;

    type IntoIter = alloc::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Array<T>
where
    T: ArrayEncodable,
{
    type Item = &'a T;

    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
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

impl<T> From<Array<T>> for Box<[T]>
where
    T: ArrayEncodable,
{
    fn from(value: Array<T>) -> Self {
        value.0
    }
}

impl<T: ArrayEncodable + Debug> Array<T> {
    /// Get the length of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use mtp_spec::object::Array;
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
    /// use mtp_spec::object::Array;
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
    /// use mtp_spec::object::Array;
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

    /// Returns a slice of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use mtp_spec::object::Array;
    ///
    /// let array = Array::from(vec![1, 2, 3]);
    /// assert_eq!(array.as_slice(), &[1, 2, 3]);
    /// ```
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}

/// Marker trait for types that can be encoded in an [`Array`].
pub trait ArrayEncodable: for<'a> DekuReader<'a, Endian> + DekuWriter<Endian> {}

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

#[cfg(test)]
mod tests {
    use super::Array;

    use alloc::vec;
    use alloc::vec::Vec;

    use deku::{DekuContainerRead, DekuContainerWrite};

    #[test_log::test]
    fn serialize_array() {
        macro_rules! test_for_type {
			($t:ty, [$($v:expr),+]) => {
				{
					let array = Array::from(vec![$($v),+]);
					let serialized = match array.to_bytes() {
						Ok(serialized) => serialized,
						Err(e) => panic!("failed to serialize type `{}`: {}", stringify!($t), e),
					};
					assert_eq!(serialized, {
						let mut bytes = Vec::new();
						bytes.extend_from_slice(&3u32.to_be_bytes());
						$(
							let elem_bytes = $v.to_be_bytes();
							bytes.extend_from_slice(elem_bytes.as_ref());
						)+
						bytes
					}, "failed to serialize type: {}", stringify!($t));
				}
			};
		}

        test_for_type!(u8, [1u8, 2u8, 3u8]);
        test_for_type!(u16, [1u16, 2u16, 3u16]);
        test_for_type!(u32, [1u32, 2u32, 3u32]);
        test_for_type!(u64, [1u64, 2u64, 3u64]);
        test_for_type!(u128, [1u128, 2u128, 3u128]);
        test_for_type!(i8, [1i8, 2i8, 3i8]);
        test_for_type!(i16, [1i16, 2i16, 3i16]);
        test_for_type!(i32, [1i32, 2i32, 3i32]);
        test_for_type!(i64, [1i64, 2i64, 3i64]);
        test_for_type!(i128, [1i128, 2i128, 3i128]);
    }

    #[test_log::test]
    fn deserialize_array() {
        macro_rules! test_for_type {
			($t:ty, [$($v:expr),+]) => {
				{
					let serialized = {
						let mut bytes = Vec::new();
						bytes.extend_from_slice(&3u32.to_be_bytes());
						$(
							let elem_bytes = $v.to_be_bytes();
							bytes.extend_from_slice(elem_bytes.as_ref());
						)+
						bytes
					};
					let (_, array) = Array::<$t>::from_bytes((serialized.as_slice(), 0)).unwrap();

					assert_eq!(array.len(), 3, "length mismatch");
					assert_eq!(array.as_slice(), [$($v),+]);
				}
			};
		}

        test_for_type!(u8, [1u8, 2u8, 3u8]);
        test_for_type!(u16, [1u16, 2u16, 3u16]);
        test_for_type!(u32, [1u32, 2u32, 3u32]);
        test_for_type!(u64, [1u64, 2u64, 3u64]);
        test_for_type!(u128, [1u128, 2u128, 3u128]);
        test_for_type!(i8, [1i8, 2i8, 3i8]);
        test_for_type!(i16, [1i16, 2i16, 3i16]);
        test_for_type!(i32, [1i32, 2i32, 3i32]);
        test_for_type!(i64, [1i64, 2i64, 3i64]);
        test_for_type!(i128, [1i128, 2i128, 3i128]);
    }
}
