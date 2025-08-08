use flatbuffers::{Vector, WIPOffset};

pub trait Builder<'a> {
    fn create_string(&mut self, s: &str) -> WIPOffset<&'a str>;
    fn raw_builder(&mut self) -> &mut flatbuffers::FlatBufferBuilder<'a>;
}

pub trait FlatSerialize<'b, B: Builder<'b>>: Sized {
    type Output: Sized + Clone + flatbuffers::Push + 'b;
    fn serialize(&self, builder: &mut B) -> Self::Output;
}

impl<'b, B: Builder<'b>> FlatSerialize<'b, B> for String {
    type Output = WIPOffset<&'b str>;
    fn serialize(&self, builder: &mut B) -> Self::Output {
        builder.create_string(self)
    }
}

impl<'b, B: Builder<'b>> FlatSerialize<'b, B> for u32 {
    type Output = u32;
    fn serialize(&self, _builder: &mut B) -> Self::Output {
        *self
    }
}

impl<'b, B: Builder<'b>> FlatSerialize<'b, B> for u64 {
    type Output = u64;
    fn serialize(&self, _builder: &mut B) -> Self::Output {
        *self
    }
}

impl<'b, B: Builder<'b>, T: 'b> FlatSerialize<'b, B> for WIPOffset<T> {
    type Output = WIPOffset<T>;
    fn serialize(&self, _builder: &mut B) -> Self::Output {
        *self
    }
}

impl<'b, B: Builder<'b>, T: FlatSerialize<'b, B>> FlatSerialize<'b, B> for Vec<T> {
    type Output =
        WIPOffset<Vector<'b, <<T as FlatSerialize<'b, B>>::Output as flatbuffers::Push>::Output>>;
    fn serialize(&self, builder: &mut B) -> Self::Output {
        let v = self
            .iter()
            .map(|x| FlatSerialize::serialize(x, builder))
            .collect::<Vec<_>>();
        builder.raw_builder().create_vector(&v)
    }
}

impl<'b, B: Builder<'b>, T: FlatSerialize<'b, B> + std::hash::Hash + Eq + Ord> FlatSerialize<'b, B>
    for std::collections::HashSet<T>
{
    type Output =
        WIPOffset<Vector<'b, <<T as FlatSerialize<'b, B>>::Output as flatbuffers::Push>::Output>>;
    fn serialize(&self, builder: &mut B) -> Self::Output {
        let mut items = self.iter().collect::<Vec<_>>();
        items.sort_unstable();
        let v = items
            .into_iter()
            .map(|x| FlatSerialize::serialize(x, builder))
            .collect::<Vec<_>>();
        builder.raw_builder().create_vector(&v)
    }
}
