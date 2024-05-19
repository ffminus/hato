hato
====

Heterogeneous Arenas of Trait Objects.

The collection is tied to a user trait, and elements are retrieved as trait objects.
This is an alternative to `Vec<Box<dyn Trait>>`, without requiring one allocation per entry.

Elements must implement the [Unscrupulous][`unscrupulous::Unscrupulous`] trait,
which makes cloning the arena fast, just a couple of `memcpy` calls per type of objects
stored in the collection. Be aware that this is quite constraining; make sure your types
fulfill the [requirements for the trait][`unscrupulous::Unscrupulous`].

Typical usage looks like this:

```rust
/// Arena will dole out elements as trait objects with this interface.
trait AsI64 {
    fn as_i64(&self) -> i64;
}

// Let's have some types implement our trait
impl<T: Copy + Into<i64>> AsI64 for T {
    fn as_i64(&self) -> i64 {
        (*self).into()
    }
}

// Declare types for are arena, and the corresponding handles
hato::hato!(AsI64, Arena, Handle, with_aba = true);

fn main() {
    // Initialize the collection
    let mut arena = Arena::default();

    // Insert a elements of different types that all implement our trait
    let x = arena.push(4_u16);
    let y = arena.push(2_i32);

    // We use the handles to access the trait objects
    assert_eq!(arena.get(x).as_i64(), 4);
    assert_eq!(arena.get(y).as_i64(), 2);
}
```

As with to bump allocators, [`Drop`] implementations will **not** be invoked on deallocation.
If you need to run the logic contained in destructors,
you can acquire a mutable reference with `get_mut`, and then call [`core::ptr::drop_in_place`].

The macro offers some configuration options via named parameters:
- vis: visibility of the declared types; private by default
- mod: name of the module used to declare the types; defaults to "_hato_mod"
