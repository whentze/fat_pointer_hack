//! # The Fat Pointer Hack
//!
//! This crate showcases a silly hack to make user-defined fat pointers
//!
//! ## What?
//!
//! In the simplest case, you can use this to tag arbitrary references with a `usize`:
//! ```
//! use fat_pointer_hack::{RefExt, FatRefExt};
//! // Create a string
//! let mut x = "Reference me!";
//!
//! // Create a tagged reference to it.
//! // Note the type annotation: it really is just a reference.
//! let mut fat_ref : &_ = (&x).tag(9001);
//!
//! // You can access the tag
//! assert_eq!(fat_ref.tag(), 9001);
//!
//! // And change it too
//! fat_ref.set_tag(1337);
//! assert_eq!(fat_ref.tag(), 1337);
//!
//! // Or turn it back into an ordinary ref
//! let regular_ref : &str = fat_ref.to_plain();
//! assert_eq!(regular_ref, "Reference me!");
//! ```
//!
//!
//! ## Why?
//!
//! In today's Rust, it is reasonably simple to write a Type that behaves like a regular reference
//! but carries some extra information:
//! ```
//! struct TaggedRef<'a, T: 'a> {
//!     reference: &'a T,
//!     tag: u32,
//! }
//! # use std::ops::Deref;
//! impl<'a, T> Deref for TaggedRef<'a, T> {
//!     type Target = T;
//!     fn deref(&self) -> &T {
//!         self.reference
//!     }
//! }
//! ```
//!
//! However, these user-defined reference types do not enjoy some of the privileges of Rust's "native" references,
//! including, but not limited to:
//! - `TaggedRef<Self>` [can not be the receiver of a method][selftypes], only `Self`, `&Self` and `&mut Self` can.
//! - Many functions and trait signatures in `std` explicitly require `&T` or `&mut T`.
//!   For example, `Index` is not allowed to return anything but `&T`.
//! - A `&mut T` will automatically coerce to a `&T` as needed.
//!
//! [selftypes]: https://github.com/rust-lang/rust/issues/27941
//!
//! ## How?
//!
//! Dynamically Sized Types such as `[T]` and `dyn Debug` have the magic property that all references to them are actually
//! twice as large as regular references.
//! We can turn any ordinary `Sized` type into a DST by tacking on a `[()]` at the end.
//! Suddenly, all references to this type are actually slices.
//! Because we don't actually care about the contents of the `[()]`, we can abuse the "length" part of that slice by
//! writing whatever we want to it.
//! Using privacy, we can make sure that nobody ever uses that "slice" as an actual slice.
//!
//! ## Is this a good idea?
//!
//! Probably not.
//!
//! ## Does Rust guarantee this is ok to do?
//!
//! No. I think this corner of Rust is currently way too underspecified to make such guarantees.
//!
//! ## Then why aren't these functions marked `unsafe`?
//!
//! `unsafe` in an API means "This can cause UB if you don't obey these invariants".
//! However, with this crate, there is no wrong way to use it and no invariants to obey.
//! If what this crate does is UB, it is UB no matter how it's used.
//!

#![no_std]

/// A fat reference to a `P` that carries a `&P` and an arbitrary usize tag.
pub type FatRef<'a, P> = &'a FatPointee<P>;

/// A mutable fat reference to a `P` that carries a `&mut P` and an arbitrary usize tag.
pub type FatRefMut<'a, P> = &'a mut FatPointee<P>;

/// Am opaque wrapper around `P` that is unsized.
/// 
/// Not used directly, but `FatRef<P>` and `FatRefMut<P>` point at this
/// and since they're just type aliases, this has to be public as well.
#[repr(C)]
pub struct FatPointee<P> {
    pointee: P,
    unsize: [()],
}

/// An extension trait for methods on FatRef
/// 
/// This needs to be an extension trait since there can't be any inherent methods on reference types.
pub trait FatRefExt<'a> {
    type Target;
    fn from_ref(thin_ref: &Self::Target, tag: usize) -> Self;
    fn to_plain(self) -> &'a Self::Target;
    fn tag(self) -> usize;
    fn set_tag(&mut self, tag: usize);
}

impl<'a, P> FatRefExt<'a> for FatRef<'a, P> {
    type Target = P;
    /// Makes a FatRef from a given reference and a tag.
    fn from_ref(thin_ref: &P, tag: usize) -> Self {
        unsafe {
            &*(core::slice::from_raw_parts(thin_ref as *const P as *const (), tag) as *const [()]
                as *const FatPointee<P>)
        }
    }

    /// Turns this FatRef back into a regular reference.
    fn to_plain(self) -> &'a P {
        &self.pointee
    }

    /// Returns the tag of this FatRef
    fn tag(self) -> usize {
        self.unsize.len()
    }

    /// Sets the tag of this FatRef to the given value.
    fn set_tag(&mut self, tag: usize) {
        *self = Self::from_ref(self.to_plain(), tag);
    }
}

/// An extension trait for methods on FatRefMut
/// 
/// This needs to be an extension trait since there can't be any inherent methods on reference types.
pub trait FatRefMutExt<'a> {
    type Target;
    fn from_ref_mut(thin_ref: &mut Self::Target, tag: usize) -> Self;
    fn to_plain_mut(self) -> &'a mut Self::Target;
}

impl<'a, P> FatRefMutExt<'a> for FatRefMut<'a, P> {
    type Target = P;
    /// Makes a FatRefMut from a given mutable reference and a tag.
    fn from_ref_mut(thin_ref: &mut P, tag: usize) -> Self {
        unsafe {
            &mut *(core::slice::from_raw_parts_mut(thin_ref as *mut P as *mut (), tag) as *mut [()]
                as *mut FatPointee<P>)
        }
    }
    /// Turns this FatRefMut back into a regular mutable reference.
    fn to_plain_mut(self) -> &'a mut P {
        &mut self.pointee
    }
}

mod refext;
/// An extension trait that adds a `.tag()` method to all regular references.
pub use refext::RefExt;

impl<P> core::convert::AsRef<P> for FatPointee<P> {
    fn as_ref(&self) -> &P {
        &self.pointee
    }
}

impl<P> core::convert::AsMut<P> for FatPointee<P> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.pointee
    }
}

use core::fmt::{self, Debug};

impl<P: Debug> Debug for FatPointee<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FatRef")
            .field("pointee", &self.pointee)
            .field("tag", &self.tag())
            .finish()
    }
}
