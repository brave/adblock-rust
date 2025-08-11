use crate::flatbuffers::unsafe_tools::fb_vector_to_slice;
use flatbuffers::{Follow, ForwardsUOffset, Vector};

pub(crate) trait PartitionPoint<'a, T: Ord> {
    fn lower_bound(&'a self, key: &T) -> usize;
}

fn partition_point_pod<'a, T: Ord + Follow<'a>>(
    keys: &'a Vector<'a, T>,
    pred: impl Fn(&T) -> bool,
) -> usize {
    fb_vector_to_slice(*keys).partition_point(pred)
}

fn partition_point_fbvector<'a, T: Follow<'a>>(
    keys: &'a Vector<'a, T>,
    pred: impl Fn(&T::Inner) -> bool,
) -> usize {
    let mut start = 0;
    let mut end: usize = keys.len();
    while start < end {
        let mid = (start + end) / 2;
        let mid_key = keys.get(mid);
        if pred(&mid_key) {
            start = mid + 1;
        } else {
            end = mid;
        }
    }
    end
}

impl<'a> PartitionPoint<'a, u32> for Vector<'a, u32> {
    fn lower_bound(&'a self, key: &u32) -> usize {
        partition_point_pod(self, |x| x < key)
    }
}

impl<'a> PartitionPoint<'a, u64> for Vector<'a, u64> {
    fn lower_bound(&'a self, key: &u64) -> usize {
        partition_point_pod(self, |x| x < key)
    }
}

impl<'a> PartitionPoint<'a, &str> for Vector<'a, ForwardsUOffset<&str>> {
    fn lower_bound(&self, key: &&str) -> usize {
        partition_point_fbvector(self, |x| *x < key)
    }
}
