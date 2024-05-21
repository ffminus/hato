use crate::Hato;

#[test]
fn base() {
    let mut arena = Hato::<dyn core::fmt::Debug>::default();

    let x = arena.push(9_i32);
    let y = arena.push(5_u16);

    assert_eq!(format!("{:?}", unsafe { arena.get(x) }), "9");
    assert_eq!(format!("{:?}", unsafe { arena.get(y) }), "5");
}
