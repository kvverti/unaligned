use std::{
    cell::Cell,
    cmp::Ordering,
    fmt::{Debug, Display},
    hash::Hash,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr,
};

use crate::Unaligned;

pub struct RefMut<'a, T> {
    data: ManuallyDrop<T>,
    cell: &'a UnalignedCell<T>,
}

// moves the (potentially modified) value back into unaligned storage
impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        // SAFETY: Nothing touches self.data again.
        let value = unsafe { ManuallyDrop::take(&mut self.data) };
        self.cell.0.set(Some(Unaligned::new(value)))
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefMut").field("data", &*self.data).finish()
    }
}

impl<T: Display> Display for RefMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (*self.data).fmt(f)
    }
}

/// A cell that provides unaligned storage for a value of type `T`. This type offers a more flexible shared API, at the
/// expense of thread safety. Note that this type is not necessarily zero-overhead in terms of size.
///
/// Because this type stores the inner value unaligned, care must be taken not to invoke more than one of this type's methods
/// concurrently. If concurrent access is detected, methods of this type may panic.
pub struct UnalignedCell<T>(Cell<Option<Unaligned<T>>>);

impl<T> UnalignedCell<T> {
    pub const fn new(value: T) -> Self {
        Self(Cell::new(Some(Unaligned::new(value))))
    }

    pub fn into_inner(self) -> T {
        self.0
            .into_inner()
            .expect("value was used concurrently")
            .into_inner()
    }

    pub fn borrow(&self) -> RefMut<'_, T> {
        let data = self
            .0
            .take()
            .expect("value was used concurrently")
            .into_inner();
        RefMut {
            data: ManuallyDrop::new(data),
            cell: self,
        }
    }

    pub fn get_mut(&mut self) -> &mut Unaligned<T> {
        self.0.get_mut().as_mut().unwrap()
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("UnalignedCell")
            .field(&*self.borrow())
            .finish()
    }
}

impl<T: Display> Display for UnalignedCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.borrow().fmt(f)
    }
}

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

    fn ne(&self, other: &Self) -> bool {
        if ptr::eq(self, other) {
            // if this is the same value, then we can't call borrow() twice
            let value = self.borrow();
            *value != *value
        } else {
            *self.borrow() != *other.borrow()
        }
    }
}

impl<T: Eq> Eq for UnalignedCell<T> {}

impl<T: PartialOrd> PartialOrd for UnalignedCell<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
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
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.borrow().hash(state);
    }

    fn hash_slice<H: std::hash::Hasher>(data: &[Self], state: &mut H) {
        for piece in data {
            piece.hash(state)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use crate::cell::UnalignedCell;

    #[test]
    fn alignment() {
        assert_eq!(1, mem::align_of::<UnalignedCell<u64>>());
    }
}
