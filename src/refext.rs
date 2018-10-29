use super::{FatRef, FatRefMut, FatRefExt, FatRefMutExt, Metadata};

/// An extension trait that is implemented for all references to Sized types automatically.
/// 
/// This trait allows you to write (&foo).tag(1234).
pub trait RefExt<M : Metadata>{
    type Output;
    fn tag(self, metadata: M) -> Self::Output;
}

impl<'a, T: Sized, M : 'a + Metadata> RefExt<M> for &'a T {
    type Output = FatRef<'a, T, M>;
    fn tag(self, metadata: M) -> FatRef<'a, T, M> {
        FatRef::from_ref(self, metadata)
    }
}

impl<'a, T: Sized, M : 'a + Metadata> RefExt<M> for &'a mut T {
    type Output = FatRefMut<'a, T, M>;
    fn tag(self, metadata: M) -> FatRefMut<'a, T, M> {
        FatRefMut::from_ref_mut(self, metadata)
    }
}