## The Fat Pointer Hack

This crate showcases a silly hack to make user-defined fat pointers

### What?

In the simplest case, you can use this to tag arbitrary references with a `usize`:
```rust
use fat_pointer_hack::{RefExt, FatRefExt};
// Create a string
let mut x = "Reference me!";

// Create a tagged reference to it.
// Note the type annotation: it really is just a reference.
let mut fat_ref : &_ = (&x).tag(9001);

// You can access the tag
assert_eq!(fat_ref.tag(), 9001);

// And change it too
fat_ref.set_tag(1337);
assert_eq!(fat_ref.tag(), 1337);

// Or turn it back into an ordinary ref
let regular_ref : &str = fat_ref.to_plain();
assert_eq!(regular_ref, "Reference me!");
```


### Why?

In today's Rust, it is reasonably simple to write a Type that behaves like a regular reference
but carries some extra information:
```rust
struct TaggedRef<'a, T: 'a> {
    reference: &'a T,
    tag: u32,
}
impl<'a, T> Deref for TaggedRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.reference
    }
}
```

However, these user-defined reference types do not enjoy some of the privileges of Rust's "native" references,
including, but not limited to:
- `TaggedRef<Self>` [can not be the receiver of a method][selftypes], only `Self`, `&Self` and `&mut Self` can.
- Many functions and trait signatures in `std` explicitly require `&T` or `&mut T`.
  For example, `Index` is not allowed to return anything but `&T`.
- A `&mut T` will automatically coerce to a `&T` as needed.

[selftypes]: https://github.com/rust-lang/rust/issues/27941

### How?

Dynamically Sized Types such as `[T]` and `dyn Debug` have the magic property that all references to them are actually
twice as large as regular references.
We can turn any ordinary `Sized` type into a DST by tacking on a `[()]` at the end.
Suddenly, all references to this type are actually slices.
Because we don't actually care about the contents of the `[()]`, we can abuse the "length" part of that slice by
writing whatever we want to it.
Using privacy, we can make sure that nobody ever uses that "slice" as an actual slice.

### Is this a good idea?

Probably not.

### Does Rust guarantee this is ok to do?

No. I think this corner of Rust is currently way too underspecified to make such guarantees.

### Then why aren't these functions marked `unsafe`?

`unsafe` in an API means "This can cause UB if you don't obey these invariants".
However, with this crate, there is no wrong way to use it and no invariants to obey.
If what this crate does is UB, it is UB no matter how it's used.

