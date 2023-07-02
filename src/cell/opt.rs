use crate::Unaligned;

/// An option enum with a well-defined representation, for accessing the field of the data-carrying variant
/// without taking a reference.
#[repr(u8)]
#[derive(Debug, Default, Copy)]
pub(super) enum OptUnaligned<T> {
    #[default]
    None,
    Some(Unaligned<T>),
}

impl<T: Copy> Clone for OptUnaligned<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> OptUnaligned<T> {
    pub(super) const fn some(value: T) -> Self {
        Self::Some(Unaligned::new(value))
    }

    pub(super) fn into_option(self) -> Option<Unaligned<T>> {
        match self {
            Self::Some(v) => Some(v),
            Self::None => None,
        }
    }

    pub(super) fn as_option_mut(&mut self) -> Option<&mut Unaligned<T>> {
        match self {
            Self::Some(v) => Some(v),
            Self::None => None,
        }
    }

    /// Get a raw pointer to the data. This function can be used to get a pointer to the interior data
    /// without going through an intermediate reference (and possibly invalidating foreign borrows).
    pub(super) unsafe fn project_ptr(this: *mut Self) -> *mut T {
        // SAFETY: Caller has guaranteed that the this ptr is valid. We know that the data is stored at offset 1
        // because of repr(u8), and that the T value is at offset 0 inside Unaligned<T>.
        unsafe { this.cast::<u8>().add(1).cast() }
    }
}
