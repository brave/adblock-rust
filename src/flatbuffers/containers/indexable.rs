use flatbuffers::{Follow, Vector};

pub(crate) trait Indexable<I> {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> I;
    fn partition_point<F>(&self, predicate: F) -> usize
    where
        F: FnMut(&I) -> bool;
}

impl<I: Copy> Indexable<I> for &[I] {
    #[inline(always)]
    fn len(&self) -> usize {
        <[I]>::len(self)
    }

    #[inline(always)]
    fn get(&self, index: usize) -> I {
        self[index]
    }

    #[inline(always)]
    fn partition_point<F>(&self, predicate: F) -> usize
    where
        F: FnMut(&I) -> bool,
    {
        <[I]>::partition_point(self, predicate)
    }
}

impl<'a, T: Follow<'a>> Indexable<T::Inner> for Vector<'a, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Vector::len(self)
    }

    #[inline(always)]
    fn get(&self, index: usize) -> T::Inner {
        Vector::get(self, index)
    }

    fn partition_point<F>(&self, mut predicate: F) -> usize
    where
        F: FnMut(&T::Inner) -> bool,
    {
        let mut left = 0;
        let mut right = self.len();

        while left < right {
            let mid = left + (right - left) / 2;
            let value = self.get(mid);
            if predicate(&value) {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        left
    }
}
