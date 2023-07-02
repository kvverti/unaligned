//! A `#![no_std]` crate containing types for encapsulating unaligned values.
//! 
//! The primary type this crate provides is [`Unaligned<T>`], which stores a value of type `T` unaligned. 
//! Storing values unaligned can be an alternative to using `#[repr(packed)]` structs (which can only be used
//! unsafely) or carefully-sized byte arrays. The [`Unaligned<T>`] type exposes a safe mutable API for working
//! with unaligned values, while the companion type [`UnalignedCell<T>`] is an interior mutability type that
//! provides a shared mutable API for working with unaligned values.
//! 
//! Because references are required to be aligned, it is unsafe to take a reference to a potentially unaligned value.
//! The `Unaligned<T>` type therefore encapsulates the unaligned-ness of the value, letting code take (safe) references
//! to `Unaligned<T>`. It also provides safe mutable APIs to set, replace, and access the value without taking an unaligned
//! reference.
//! 
//! Note that, in general, a safe *shared* API is not possible for unaligned values, because in order to do anything useful
//! with an unaligned value, the value must be moved into aligned storage - an inherently exclusive operation. The
//! `UnalignedCell<T>` type somewhat alleviates this restriction by allowing exclusive access through a shared API
//! using the power of interior mutability.
//! 
//! This crate is `#![no_std]` by default. The `std` feature can be enabled to access functionality that requires the full
//! standard library.
//! 
//! [`UnalignedCell<T>`]: self::cell::UnalignedCell

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "std")]
extern crate std;

pub mod unaligned;
pub mod cell;

pub use self::unaligned::Unaligned;
