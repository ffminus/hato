use core::mem::size_of;

trait AsI64 {
    fn as_i64(&self) -> i64;
}

impl<T: Copy + Into<i64>> AsI64 for T {
    fn as_i64(&self) -> i64 {
        (*self).into()
    }
}

crate::hato!(AsI64, ArenaThin, HandleThin, with_aba = true, mod = _thin);

#[test]
fn thin() {
    assert_eq!(size_of::<HandleThin>(), 8);

    let mut arena = ArenaThin::default();

    let x = arena.push(9_i32);
    let y = arena.push(5_u16);

    assert_eq!(arena.get_mut(x).as_i64(), 9);
    assert_eq!(arena.get_mut(y).as_i64(), 5);
}

crate::hato!(AsI64, ArenaWide, HandleWide, with_aba = true, wide_handles = true, mod = _wide);

#[test]
fn wide() {
    assert_eq!(size_of::<HandleWide>(), 16);

    let mut arena = ArenaWide::default();

    let x = arena.push(9_i32);
    let y = arena.push(5_u16);

    assert_eq!(arena.get_mut(x).as_i64(), 9);
    assert_eq!(arena.get_mut(y).as_i64(), 5);
}
