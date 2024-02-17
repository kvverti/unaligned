use core::{
    cell::Cell,
    cmp::Ordering,
    fmt::{Debug, Display},
    hash::Hash,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    ptr,
};

use crate::Unaligned;

use self::opt::OptUnaligned;

/// Private module that defines an option type for use in the cell.
mod opt;

/// A value borrowed from an [`UnalignedCell`].
pub struct RefMut<'a, T> {
    data: ManuallyDrop<T>,
    cell: &'a UnalignedCell<T>,
}

// moves the (potentially modified) value back into unaligned storage
impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        // SAFETY: Nothing touches self.data again.
        let value = unsafe { ManuallyDrop::take(&mut self.data) };
        self.cell.0.set(OptUnaligned::some(value))
    }
}

impl<T> Deref for RefMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: Debug> Debug for RefMut<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RefMut").field("data", &*self.data).finish()
    }
}

impl<T: Display> Display for RefMut<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        (*self.data).fmt(f)
    }
}

#[derive(Debug)]
pub struct BorrowError;

#[cfg(feature = "std")]
impl std::error::Error for BorrowError {}

impl Display for BorrowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("BorrowError")
    }
}

/// A cell that provides unaligned storage for a value of type `T`. This type offers a more flexible shared API, at the
/// expense of thread safety. Note that this type is not necessarily zero-overhead in terms of size.
///
/// Because this type only allows exclusive access to its contents, care must be taken not to borrow the contents more than once
/// concurrently. If concurrent access is detected, methods of this type will panic.
pub struct UnalignedCell<T>(Cell<OptUnaligned<T>>);

impl<T> UnalignedCell<T> {
    /// Construct a new `UnalignedCell` that wraps the given value.
    pub const fn new(value: T) -> Self {
        Self(Cell::new(OptUnaligned::some(value)))
    }

    /// Consume this cell and return its contents.
    pub fn into_inner(self) -> T {
        self.0
            .into_inner()
            .into_option()
            .expect("value should not be borrowed (was a borrow leaked?)")
            .into_inner()
    }

    /// Get a raw pointer to the contents of this cell. Note that if the contents are borrowed, then the returned pointer will
    /// be invalid until the borrow is relinquished.
    ///
    /// **Caution:** The returned pointer is almost certainly unaligned. You should only perform operations that
    /// are safe with unaligned pointers (e.g. [`write_unaligned`]). Dereferencing the returned pointer is almost certainly
    /// _undefined behavior_.
    ///
    /// [`write_unaligned`]: https://doc.rust-lang.org/beta/core/primitive.pointer.html#method.write_unaligned
    pub fn as_ptr(&self) -> *mut T {
        // SAFETY: The pointer points to a valid OptUnaligned<T> value.
        unsafe { OptUnaligned::project_ptr(self.0.as_ptr()) }
    }

    /// Mutably borrow the contents of this cell. The contents cannot be borrowed again until the returnd `RefMut` is destroyed.
    ///
    /// ## Panics
    /// This method panics if the contents are currently borrowed.
    pub fn borrow(&self) -> RefMut<'_, T> {
        self.try_borrow().expect("value should not be borrowed")
    }

    /// Mutably borrow the contents of this cell. If the contents are already borrowed, this method returns an error.
    /// 
    /// ## Example
    /// ```
    /// # use unaligned::cell::UnalignedCell;
    /// let cell = UnalignedCell::new(42);
    /// let first_borrow = cell.try_borrow().expect("there isn't a borrow yet");
    /// assert_eq!(42, *first_borrow);
    /// 
    /// let second_borrow = cell.try_borrow();
    /// assert!(second_borrow.is_err());
    /// ```
    pub fn try_borrow(&self) -> Result<RefMut<'_, T>, BorrowError> {
        let data = self.0.take().into_option().ok_or(BorrowError)?.into_inner();
        Ok(RefMut {
            data: ManuallyDrop::new(data),
            cell: self,
        })
    }

    /// Get a mutable reference to the unaligned contents. Because this method takes `self` by mutable reference,
    /// no runtime checks are needed.
    /// 
    /// ## Example
    /// ```
    /// # use unaligned::cell::UnalignedCell;
    /// let mut cell = UnalignedCell::new(42);
    /// assert_eq!(42, cell.get_mut().get());
    /// ```
    pub fn get_mut(&mut self) -> &mut Unaligned<T> {
        self.0.get_mut().as_option_mut().unwrap()
    }

    /// Swaps the contents of this cell with the contents of another.
    /// 
    /// ## Panics
    /// This method panics if either value is already borrowed, or if both arguments refer to the same cell.
    pub fn swap(&self, other: &Self) {
        mem::swap(&mut *self.borrow(), &mut *other.borrow());
    }

    /// Replace the contents of this cell with the given value, and return the previous value.
    /// 
    /// ## Panics
    /// This method panics if the value is already borrowed.
    pub fn replace(&self, value: T) -> T {
        mem::replace(&mut self.borrow(), value)
    }

    /// Replace the contents of this cell using the given function to produce a new value. The previous value
    /// is returned.
    /// 
    /// ## Panics
    /// This method panics if the value is already borrowed.
    /// 
    /// ## Example
    /// ```
    /// # use unaligned::cell::UnalignedCell;
    /// let cell = UnalignedCell::new(42);
    /// let original = cell.replace_with(|val| *val + 28);
    /// assert_eq!(42, original);
    /// assert_eq!(UnalignedCell::new(70), cell);
    /// ```
    pub fn replace_with<F>(&self, f: F) -> T
    where
        F: FnOnce(&mut T) -> T,
    {
        let mut val = self.borrow();
        let new_val = f(&mut val);
        mem::replace(&mut val, new_val)
    }
}

impl<T: Default> UnalignedCell<T> {
    /// Get the contents of this cell. The default value of type `T` is left in the cell.
    pub fn take(&self) -> T {
        self.replace(T::default())
    }
}

// trait implementations

impl<T> From<T> for UnalignedCell<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Clone> Clone for UnalignedCell<T> {
    fn clone(&self) -> Self {
        Self::new(self.borrow().clone())
    }
}

impl<T: Default> Default for UnalignedCell<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Debug> Debug for UnalignedCell<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("UnalignedCell")
            .field(&*self.borrow())
            .finish()
    }
}

impl<T: Display> Display for UnalignedCell<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.borrow().fmt(f)
    }
}

#[allow(clippy::eq_op)] // self-comparisons necessary to forward behavior
impl<T: PartialEq> PartialEq for UnalignedCell<T> {
    fn eq(&self, other: &Self) -> bool {
        if ptr::eq(self, other) {
            // if this is the same value, then we can't call borrow() twice
            let value = self.borrow();
            *value == *value
        } else {
            *self.borrow() == *other.borrow()
        }
    }
}

impl<T: Eq> Eq for UnalignedCell<T> {}

#[allow(clippy::eq_op)] // self-comparisons necesary to forward behavior
impl<T: PartialOrd> PartialOrd for UnalignedCell<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        if ptr::eq(self, other) {
            let value = self.borrow();
            value.partial_cmp(&value)
        } else {
            self.borrow().partial_cmp(&other.borrow())
        }
    }

    fn lt(&self, other: &Self) -> bool {
        if ptr::eq(self, other) {
            let value = self.borrow();
            *value < *value
        } else {
            *self.borrow() < *other.borrow()
        }
    }

    fn le(&self, other: &Self) -> bool {
        if ptr::eq(self, other) {
            let value = self.borrow();
            *value <= *value
        } else {
            *self.borrow() <= *other.borrow()
        }
    }

    fn gt(&self, other: &Self) -> bool {
        if ptr::eq(self, other) {
            let value = self.borrow();
            *value > *value
        } else {
            *self.borrow() > *other.borrow()
        }
    }

    fn ge(&self, other: &Self) -> bool {
        if ptr::eq(self, other) {
            let value = self.borrow();
            *value >= *value
        } else {
            *self.borrow() >= *other.borrow()
        }
    }
}

impl<T: Ord> Ord for UnalignedCell<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        if ptr::eq(self, other) {
            let value = self.borrow();
            value.cmp(&value)
        } else {
            self.borrow().cmp(&other.borrow())
        }
    }

    fn max(self, other: Self) -> Self {
        Self::new(T::max(self.into_inner(), other.into_inner()))
    }

    fn min(self, other: Self) -> Self {
        Self::new(T::min(self.into_inner(), other.into_inner()))
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        Self::new(T::clamp(
            self.into_inner(),
            min.into_inner(),
            max.into_inner(),
        ))
    }
}

impl<T: Hash> Hash for UnalignedCell<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.borrow().hash(state);
    }
}
