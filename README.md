hato
====

Heterogeneous Arenas of Trait Objects.

The collection is tied to a user trait, and elements are retrieved as trait objects.
This is an alternative to `Vec<Box<dyn Trait>>`, without requiring one allocation per entry.
A bump allocator like [`bumpalo`](https://docs.rs/bumpalo/latest/bumpalo) will bring
similar benefits, but will not offer methods for memory reclamation.
This can be limiting for workloads involving alternating insertions and deletions.

Elements must implement the [Unscrupulous][`unscrupulous::Unscrupulous`] trait,
which makes cloning the arena fast, just a couple of `memcpy` calls per type of objects
stored in the collection. Be aware that this is quite constraining; make sure your types
fulfill the [requirements for the trait][`unscrupulous::Unscrupulous`].

Typical usage looks like this:

```rust
// Initialize the collection
let mut arena = hato::Hato::<dyn core::fmt::Debug>::default();

// Insert a elements of different types that all implement our trait
let x = arena.push(4_u16);
let y = arena.push(2_i32);

// We use the handles to access the trait objects
assert_eq!(format!("{:?}", arena.get(x)), "4");
assert_eq!(format!("{:?}", arena.get(y)), "2");

// We can remove individual elements...
arena.remove(x);

// ... and re-use the underlying capacity
let _z = arena.push(7_u16);
```


Caveats
-------
- This crate requires unstable features, stay on version 0.1.0 if you cannot use nightly.
- `Hato` groups objects by their virtual table, which is [duplicated across codegen units](https://doc.rust-lang.org/std/ptr/struct.DynMetadata.html). Building with `codegen-units = 1` can be worthwhile to reduce the number of separate arenas.
- This collection is subject to the [ABA problem](https://en.wikipedia.org/wiki/ABA_problem). See type documentation for more details.


Acknowledgements
----------------

Major thanks to [@bluurryy](https://github.com/bluurryy)! Without their skills and time,
this crate would have stayed a messy, unwieldy, non-generic, unergonomic macro.
