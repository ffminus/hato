//! Benchmark common operations, contrasting basic vectors of boxes with [`hato`] implementations.

use criterion::{black_box, criterion_main, Criterion};
use dyn_clone::{clone_trait_object, DynClone};
use rand::prelude::{Rng, SeedableRng, StdRng};

use hato::{Handle, Hato};

/// Trait with minimal logic, to focus benchmark on iteration and cloning.
trait AsI32: DynClone {
    fn as_i32(&self) -> i32;
}

impl<T: Copy + Into<i32> + DynClone> AsI32 for T {
    fn as_i32(&self) -> i32 {
        (*self).into()
    }
}

// Ensure trait objects can be cloned, as this is the main use case of `hato`
clone_trait_object!(AsI32);

/// Fill vector and arena with a large number of trait objects.
const N: usize = 100_000;

/// Declare benchmarks for specific test instances
fn generate_then_benchmark(c: &mut Criterion) {
    // Ensure reproducibility of benchmark
    let mut rng = StdRng::seed_from_u64(0);

    benchmark(c, "with_aba", generate(&mut rng));
}

type Inputs = (Vec<Box<dyn AsI32>>, Hato<dyn AsI32>, Vec<Handle>);

fn generate(rng: &mut StdRng) -> Inputs {
    // Basic approach of a vector of boxed trait objects
    let mut boxes = Vec::<Box<dyn AsI32>>::new();

    // Heterogeneous arena, with an external vector for handles
    let (mut arena, mut handles) = (Hato::default(), Vec::new());

    // Fill collections with trait objects with randomized types
    for i in 0..N {
        if rng.gen::<bool>() {
            #[allow(clippy::cast_possible_truncation)]
            let i = i as u8;

            boxes.push(Box::new(i));
            handles.push(arena.push(i));
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let i = i as i8;

            boxes.push(Box::new(i));
            handles.push(arena.push(i));
        }
    }

    (boxes, arena, handles)
}

fn benchmark(c: &mut Criterion, name: &str, (boxes, arena, handles): Inputs) {
    let _c = c.bench_function(&format!("{name} iterate boxes"), |b| {
        b.iter(|| black_box(&boxes).iter().map(|b| b.as_i32()).sum::<i32>());
    });

    let _c = c.bench_function(&format!("{name} iterate arena"), |b| {
        b.iter(|| sum_arena(black_box((&arena, &handles))));
    });

    let handles_sorted = sort_handles_by_type_and_offset(&handles);

    let _c = c.bench_function(&format!("{name} iterate arena sorted"), |b| {
        b.iter(|| sum_arena(black_box((&arena, &handles_sorted))));
    });

    let _c = c.bench_function(&format!("{name} clone   boxes"), |b| {
        b.iter(|| Clone::clone(black_box(&boxes)));
    });

    let _c = c.bench_function(&format!("{name} clone   arena"), |b| {
        b.iter(|| {
            drop(Clone::clone(black_box(&arena)));
            drop(Clone::clone(black_box(&handles)));
        });
    });
}

/// Sum all elements stored in the arena.
fn sum_arena((arena, handles): (&Hato<dyn AsI32>, &[Handle])) -> i32 {
    handles
        .iter()
        .map(|h| unsafe { arena.get(*h) }.as_i32())
        .sum()
}

/// Sorting helps with the jump target prediction and cache.
fn sort_handles_by_type_and_offset(handles: &[Handle]) -> Vec<Handle> {
    let mut handles = handles.to_vec();
    handles.sort();
    handles
}

// ? Wrap benchmark group declaration to fix missing documentation lint
mod groups {
    criterion::criterion_group!(benches, super::generate_then_benchmark);
}

// Benchmark harness
criterion_main!(groups::benches);
