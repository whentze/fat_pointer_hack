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
//! let mut x = 5;
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
//! let regular_ref : &u32 = fat_ref.to_plain();
//! assert_eq!(*regular_ref, 5);
//! ```
//! 
//! You can also tag with other types such as floats or chars:
//! ```
//! # use fat_pointer_hack::{RefExt, FatRefExt};
//! let mut x = "Rust";
//!
//! let mut heart_ref = (&x).tag('♥');
//! let mut float_ref = (&x).tag(0.9);
//!
//! assert_eq!(heart_ref.tag(), '♥');
//! assert_eq!(float_ref.tag(), 0.9);
//! ```
//! 
//! Note that these obey the regular borrowing rules:
//! ```compile_fail
//! # use fat_pointer_hack::{RefExt, FatRefExt};
//! let mut x = vec![1,2,3];
//! 
//! let shared_fat_ref = (&x).tag(0);
//! 
//! x.push(4); // Doesn't compile - x is borrowed!
//! ```
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
pub type FatRef<'a, P, M> = &'a FatPointee<P, M>;

/// A mutable fat reference to a `P` that carries a `&mut P` and an arbitrary usize tag.
pub type FatRefMut<'a, P, M> = &'a mut FatPointee<P, M>;

/// Am opaque wrapper around `P` that is unsized.
///
/// Not used directly, but `FatRef<P>` and `FatRefMut<P>` point at this
/// and since they're just type aliases, this has to be public as well.
#[repr(C)]
pub struct FatPointee<P, M> {
    pointee: P,
    phantom: core::marker::PhantomData<M>,
    unsize: [()],
}

pub struct Tag(usize);

/// A trait for types that can be used as a Tag.
pub trait Metadata: Sized {
    /// Stuff this value into a Tag.
    fn pack(self) -> Tag;
    /// Unpack this value from a Tag.
    fn unpack(Tag) -> Self;
}

impl Metadata for usize {
    fn pack(self) -> Tag {
        Tag(self)
    }
    fn unpack(val: Tag) -> Self {
        val.0
    }
}

impl Metadata for [u8; core::mem::size_of::<usize>()] {
    fn pack(self) -> Tag {
        Tag(unsafe { core::mem::transmute(self) })
    }
    fn unpack(val: Tag) -> Self {
        unsafe { core::mem::transmute(val.0) }
    }
}

#[cfg(target_pointer_width = "64")]
impl Metadata for f64 {
    fn pack(self) -> Tag {
        Tag(self.to_bits() as usize)
    }
    fn unpack(val: Tag) -> Self {
        Self::from_bits(val.0 as u64)
    }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl Metadata for f32 {
    fn pack(self) -> Tag {
        Tag(self.to_bits() as usize)
    }
    fn unpack(val: Tag) -> Self {
        Self::from_bits(val.0 as u32)
    }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl Metadata for char {
    fn pack(self) -> Tag {
        Tag(unsafe{core::mem::transmute::<char, u32>(self)} as usize)
    }
    fn unpack(val: Tag) -> Self {
        unsafe{core::mem::transmute(val.0 as u32)}
    }
}

/// An extension trait for methods on FatRef
///
/// This needs to be an extension trait since there can't be any inherent methods on reference types.
pub trait FatRefExt<'a> {
    type Target;
    type Meta: Metadata;
    fn from_ref(thin_ref: &Self::Target, metadata: Self::Meta) -> Self;
    fn to_plain(self) -> &'a Self::Target;
    fn tag(self) -> Self::Meta;
    fn set_tag(&mut self, tag: Self::Meta);
}

impl<'a, P, M: 'a + Metadata> FatRefExt<'a> for FatRef<'a, P, M> {
    type Target = P;
    type Meta = M;
    /// Makes a FatRef from a given reference and a tag.
    fn from_ref(thin_ref: &P, tag: M) -> Self {
        unsafe {
            &*(core::slice::from_raw_parts(thin_ref as *const P as *const (), tag.pack().0)
                as *const [()] as *const FatPointee<P, M>)
        }
    }

    /// Turns this FatRef back into a regular reference.
    fn to_plain(self) -> &'a P {
        &self.pointee
    }

    /// Returns the tag of this FatRef
    fn tag(self) -> M {
        M::unpack(Tag(self.unsize.len()))
    }

    /// Sets the tag of this FatRef to the given value.
    fn set_tag(&mut self, tag: M) {
        *self = Self::from_ref(self.to_plain(), tag);
    }
}

/// An extension trait for methods on FatRefMut
///
/// This needs to be an extension trait since there can't be any inherent methods on reference types.
pub trait FatRefMutExt<'a> {
    type Target;
    type Meta : Metadata;
    fn from_ref_mut(thin_ref: &mut Self::Target, tag: Self::Meta) -> Self;
    fn to_plain_mut(self) -> &'a mut Self::Target;
}

impl<'a, P, M: 'a + Metadata> FatRefMutExt<'a> for FatRefMut<'a, P, M> {
    type Target = P;
    type Meta = M;
    /// Makes a FatRefMut from a given mutable reference and a tag.
    fn from_ref_mut(thin_ref: &mut P, tag: M) -> Self {
        unsafe {
            &mut *(core::slice::from_raw_parts_mut(thin_ref as *mut P as *mut (), tag.pack().0)
                as *mut [()] as *mut FatPointee<P, M>)
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

impl<P, M> core::convert::AsRef<P> for FatPointee<P, M> {
    fn as_ref(&self) -> &P {
        &self.pointee
    }
}

impl<P, M> core::convert::AsMut<P> for FatPointee<P, M> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.pointee
    }
}

use core::fmt::{self, Debug};

impl<P: Debug, M: Debug + Metadata> Debug for FatPointee<P, M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FatRef")
            .field("pointee", &self.pointee)
            .field("tag", &self.tag())
            .finish()
    }
}
