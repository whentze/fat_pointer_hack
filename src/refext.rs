use super::{FatRef, FatRefMut, FatRefExt, FatRefMutExt};

/// An extension trait that is implemented for all references to Sized types automatically.
/// 
/// This trait allows you to write (&foo).tag(1234).
pub trait RefExt{
    type Output;
    fn tag(self, metadata: usize) -> Self::Output;
}

impl<'a, T: Sized> RefExt for &'a T {
    type Output = FatRef<'a, T>;
    fn tag(self, metadata: usize) -> FatRef<'a, T> {
        FatRef::from_ref(self, metadata)
    }
}

impl<'a, T: Sized> RefExt for &'a mut T {
    type Output = FatRefMut<'a, T>;
    fn tag(self, metadata: usize) -> FatRefMut<'a, T> {
        FatRefMut::from_ref_mut(self, metadata)
    }
}