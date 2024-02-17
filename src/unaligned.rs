use core::{
    fmt::Debug,
    mem::{self, ManuallyDrop},
    ptr,
};

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
    /// ## Example
    /// ```
    /// # use unaligned::Unaligned;
    /// let unaligned: Unaligned<String> = Unaligned::new(String::from("Hello world!"));
    /// let ptr = unaligned.as_ptr();
    /// ```
    ///
    /// [`read_unaligned`]: https://doc.rust-lang.org/beta/core/primitive.pointer.html#method.read_unaligned
    pub const fn as_ptr(&self) -> *const T {
        ptr::addr_of!(self.0)
    }

    /// Get a writable pointer to the inner value.
    ///
    /// **Caution:** The returned pointer is almost certainly unaligned. You should only perform operations that
    /// are safe with unaligned pointers (e.g. [`write_unaligned`]). Dereferencing the returned pointer is almost certainly
    /// _undefined behavior_. If you simply need to borrow the inner value, consider using [`Unaligned::with_mut`].
    ///
    /// ## Example
    /// ```
    /// # use unaligned::Unaligned;
    /// let mut unaligned: Unaligned<String> = Unaligned::new(String::from("Hello"));
    /// let ptr = unaligned.as_mut_ptr();
    /// ```
    ///
    /// [`write_unaligned`]: https://doc.rust-lang.org/beta/core/primitive.pointer.html#method.write_unaligned
    pub fn as_mut_ptr(&mut self) -> *mut T {
        ptr::addr_of_mut!(self.0)
    }

    /// Create a shared reference to unaligned data from a raw pointer.
    ///
    /// ## Safety
    /// The caller must ensure that the pointer has the following properties.
    /// - The pointer must be valid for unaligned reads at type `T`.
    /// - The pointer must point to data that is valid for at least `'a`.
    /// - The pointer must not alias with any mutable borrows of the same data for `'a`.
    pub unsafe fn from_ptr<'a>(ptr: *const T) -> &'a Self {
        // SAFETY: The caller upholds the above safety invariants, which are sufficient to justify this.
        unsafe { &*ptr.cast() }
    }

    /// Create a mutable reference to unaligned data from a raw pointer.
    ///
    /// ## Safety
    /// The caller must ensure that the pointer has the following properties.
    /// - The pointer must be valid for unaligned reads and writes at type `T`.
    /// - The pointer must point to data that is valid for at least `'a`.
    /// - The pointer must not alias with any other borrows (mutable or shared) of the same data for `'a`.
    pub unsafe fn from_mut_ptr<'a>(ptr: *mut T) -> &'a mut Self {
        // SAFETY: The caller upholds the above safety invariants, which are sufficient to justify this.
        unsafe { &mut *ptr.cast() }
    }

    /// Get a shared reference to the inner value. If `self` happens to be aligned to type `T`, then this method
    /// gives direct access to the inner value.
    pub fn get_aligned(&self) -> Option<&T> {
        let data_ptr = self.as_ptr();
        if data_ptr as usize % mem::align_of::<T>() == 0 {
            // SAFETY: We have verified that the data pointer is aligned.
            Some(unsafe { &*data_ptr })
        } else {
            None
        }
    }

    /// Get a mutable reference to the inner value. If `self` happens to be alogned to type `T`, then this method
    /// gives direct access to the inner value.
    pub fn get_aligned_mut(&mut self) -> Option<&mut T> {
        let data_ptr = self.as_mut_ptr();
        if data_ptr as usize % mem::align_of::<T>() == 0 {
            // SAFETY: We have verified that the data pointer is aligned.
            Some(unsafe { &mut *data_ptr })
        } else {
            None
        }
    }

    /// Get a shared reference to the inner value without checking if it is aligned.
    /// 
    /// ## Safety
    /// The caller must ensure that the `self` pointer is aligned for `T`.
    pub unsafe fn get_aligned_unchecked(&self) -> &T {
        // SAFETY: The caller has ensured that the pointer is aligned.
        unsafe { &*self.as_ptr() }
    }

    /// Get a mutable reference to the inner value without checking if it is aligned.
    /// 
    /// ## Safety
    /// The caller must ensure that the `self` pointer is aligned for `T`.
    pub unsafe fn get_aligned_unchecked_mut(&mut self) -> &mut T {
        // SAFETY: The caller has ensured that the pointer is aligned.
        unsafe { &mut *self.as_mut_ptr() }
    }

    /// Swap the inner value of this `Unaligned` with another value.
    pub fn swap(&mut self, other: &mut T) {
        self.with_mut(|val| mem::swap(val, other));
    }

    /// Swaps the inner value of this `Unaligned` with the given value, and return the former inner value.
    pub fn replace(&mut self, value: T) -> T {
        self.with_mut(|val| mem::replace(val, value))
    }

    /// Set the inner value of this `Unaligned`.
    pub fn set(&mut self, value: T) {
        self.0 = value;
    }

    /// Mutably borrow the inner value and perform some computation with it. This is useful if you want access to the inner value,
    /// but are not able to swap it with anything.
    ///
    /// ## Why is there no `with_ref`?
    /// A shared version of this API (taking `&self`) would be _unsound_ because this method needs to move the inner value to
    /// a properly aligned location. Because shared reference APIs can be called from multiple places at the same time, two or
    /// more callers could execute a hypothetical shared reference version of this function at the same time and read
    /// unitialized/invalid data.
    /// 
    /// ## Example
    /// ```
    /// # use unaligned::Unaligned;
    /// let mut value = Unaligned::new(42);
    /// let sum = value.with_mut(|v| *v + 28);
    /// assert_eq!(70, sum);
    /// ```
    pub fn with_mut<R, F>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let data_ptr = self.as_mut_ptr();
        // SAFETY: data_ptr is only read and written to with unaligned operations. The data is always written back
        // to the unaligned storage after f exits, even under unwinding. Taking self by mutable reference ensures
        // that no one else can run this code (and take the value out from under us).
        unsafe {
            let mut guard =
                scopeguard::guard(data_ptr.read_unaligned(), |v| data_ptr.write_unaligned(v));
            f(&mut *guard)
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

impl<T, const N: usize> Unaligned<[T; N]> {
    /// Transform an unaligned array of `T` into an array of unaligned `T`.
    pub fn into_array_of_unaligned(self) -> [Unaligned<T>; N] {
        // SAFETY: Unaligned<[T; N]> and [Unaligned<T>; N] have the same size, alignment, and validity.
        unsafe {
            let this = ManuallyDrop::new(self);
            mem::transmute_copy::<Self, _>(&*this)
        }
    }

    /// View an unaligned array of `T` as an array of unaligned `T`.
    pub fn as_array_of_unaligned(&self) -> &[Unaligned<T>; N] {
        // SAFETY: Unaligned<[T; N]> and [Unaligned<T>; N] have the same size, alignment, and validity.
        unsafe { mem::transmute(self) }
    }

    /// View an unaligned array of `T` as an array of unaligned `T`.
    pub fn as_mut_array_of_unaligned(&mut self) -> &mut [Unaligned<T>; N] {
        // SAFETY: Unaligned<[T; N]> and [Unaligned<T>; N] have the same size, alignment, and validity.
        unsafe { mem::transmute(self) }
    }
}

// trait implementations

impl<T> From<T> for Unaligned<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

// we cannot soundly move out and call any shared reference API on T.
impl<T: Copy> Clone for Unaligned<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Copy> Copy for Unaligned<T> {}

impl<T> Debug for Unaligned<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Unaligned").field(&"<unaligned>").finish()
    }
}
