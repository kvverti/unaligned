use std::{ptr, mem, fmt::Debug};

/// An unaligned value of type `T`. See the crate documentation for more details.
#[repr(C, packed)]
#[derive(Default)]
pub struct Unaligned<T>(T);

impl<T> Unaligned<T> {
    /// Construct a new `Unaligned` with the given value.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Consume this `Unaligned` and return the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Get a read-only pointer to the inner value.
    /// 
    /// **Caution:** The returned pointer is almost certainly unaligned. You should only perform operations that
    /// are safe with unaligned pointers (e.g. [`read_unaligned`]). Dereferencing the returned pointer is almost certainly
    /// _undefined behavior_.
    /// 
    /// [`read_unaligned`]: https://doc.rust-lang.org/beta/core/primitive.pointer.html#method.read_unaligned
    pub const fn as_ptr(&self) -> *const T {
        ptr::addr_of!(self.0)
    }

    /// Get a writable pointer to the inner value.
    /// 
    /// **Caution:** The returned pointer is almost certainly unaligned. You should only perform operations that
    /// are safe with unaligned pointers (e.g. [`write_unaligned`]). Dereferencing the returned pointer is almost certainly
    /// _undefined behavior_.
    /// 
    /// [`write_unaligned`]: https://doc.rust-lang.org/beta/core/primitive.pointer.html#method.write_unaligned
    pub fn as_mut_ptr(&mut self) -> *mut T {
        ptr::addr_of_mut!(self.0)
    }

    /// Swap the inner value of this `Unaligned` with another value.
    pub fn swap(&mut self, other: &mut T) {
        self.with_value(|val| mem::swap(val, other));
    }

    /// Swaps the inner value of this `Unaligned` with the given value, and return the former inner value.
    pub fn replace(&mut self, mut value: T) -> T {
        self.swap(&mut value);
        value
    }

    /// Set the inner value of this `Unaligned`.
    pub fn set(&mut self, value: T) {
        self.0 = value;
    }

    pub fn get_aligned(&self) -> Option<&T> {
        let data_ptr = self.as_ptr();
        // SAFETY: We have verified that the data pointer is aligned.
        if data_ptr as usize % mem::align_of::<T>() == 0 {
            Some(unsafe { &*data_ptr })
        } else {
            None
        }
    }

    pub fn get_aligned_mut(&mut self) -> Option<&mut T> {
        let data_ptr = self.as_mut_ptr();
        // SAFETY: We have verified that the data pointer is aligned.
        if data_ptr as usize % mem::align_of::<T>() == 0 {
            Some(unsafe { &mut *data_ptr })
        } else {
            None
        }
    }

    /// Mutably borrow the inner value and perform some computation with it. This is useful if you want access to the inner value,
    /// but are not able to swap it with anything.
    /// 
    /// ### Why no shared reference version?
    /// A shared version of this API (taking `&self`) would be _unsound_ because this method needs to move the inner value to
    /// a properly aligned location. Because shared reference APIs can be called from multiple places at the same time, two or
    /// more callers could execute a hypothetical shared reference version of this function at the same time and read
    /// unitialized/invalid data.
    pub fn with_value<R, F>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let data_ptr = self.as_mut_ptr();
        // SAFETY: data_ptr is only read and written to with unaligned operations. The data is always written back
        // to the unaligned storage after f exits, even under unwinding. Taking self by mutable reference ensures
        // that no one else can run this code (and take the value out from under us).
        unsafe {
            let mut tmp = drop_guard::guard(data_ptr.read_unaligned(), |val| {
                data_ptr.write_unaligned(val);
            });
            f(&mut tmp)
        }
    }
}

impl<T: Default> Unaligned<T> {
    /// Replace the inner value of this `Unaligned` with the default value of type `T`, and return the former inner value.
    pub fn take(&mut self) -> T {
        self.replace(T::default())
    }
}

impl<T: Copy> Unaligned<T> {
    /// Copy the inner value of this `Unaligned`.
    pub const fn get(&self) -> T {
        self.0
    }
}

// trait implementations

// we cannot soundly move out and call any shared reference API on T.
impl<T: Copy> Clone for Unaligned<T> {
    fn clone(&self) -> Self {
        *self
    }

    fn clone_from(&mut self, source: &Self) {
        *self = *source;
    }
}

impl<T: Copy> Copy for Unaligned<T> {}

impl<T> Debug for Unaligned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Unaligned").field(&"<unaligned>").finish()
    }
}
