use flatbuffers::{Follow, Vector};

/// A trait to access indexed data in a flatbuffer.
/// It has two implementations:
/// 1. a faster &[I] for slices;
/// 2. a slower for flatbuffers::Vector<I>, that uses Follow() internally.
///
/// Note: it intentally returns values using a copy, because it's faster
/// than by reference.
pub(crate) trait FbIndex<I> {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> I;
}

impl<I: Copy> FbIndex<I> for &[I] {
    #[inline(always)]
    fn len(&self) -> usize {
        <[I]>::len(self)
    }

    #[inline(always)]
    fn get(&self, index: usize) -> I {
        self[index]
    }
}

impl FbIndex<()> for () {
    #[inline(always)]
    fn len(&self) -> usize {
        0
    }
    fn get(&self, _index: usize) {}
}

impl<'a, T: Follow<'a>> FbIndex<T::Inner> for Vector<'a, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Vector::len(self)
    }

    #[inline(always)]
    fn get(&self, index: usize) -> T::Inner {
        Vector::get(self, index)
    }
}
