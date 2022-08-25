#[track_caller]
pub(crate) fn collect_array<T: Default + Copy, const N: usize>(
    mut iter: impl Iterator<Item = T>,
) -> Option<[T; N]> {
    let mut out = [T::default(); N];
    for item in out.iter_mut() {
        *item = iter.next()?
    }
    Some(out)
}
