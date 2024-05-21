// Unstable features necessary to avoid macros
#![feature(ptr_metadata, unsize)]
// Use `README.md` as documentation home page, to reduce duplication
#![doc = include_str!("../README.md")]

#[cfg(test)]
mod tests;

use core::marker::Unsize;
use core::mem::align_of;
use core::ptr::{from_raw_parts, from_raw_parts_mut, from_ref, metadata, DynMetadata, Pointee};

use aligned_vec::AVec;
use unscrupulous::{as_slice_of_bytes, Unscrupulous};

/// Arenas of heterogeneous trait objects, stored by type in separate vectors.
///
/// As with bump allocators, [`Drop`] implementations will **not** be invoked on deallocation
/// or calls to `remove`. If you need to run the logic contained in destructors,
/// you can acquire a mutable reference with `get_mut`, and then call [`core::ptr::drop_in_place`].
///
/// This type is subject to the [ABA problem](https://en.wikipedia.org/wiki/ABA_problem).
/// Using handles of previously removed elements will **not** trigger errors but will return
/// stale or newly inserted elements. This can lead to unexpected behavior, as shown below:
///
/// ```rust
/// // Initialize the collection
/// let mut arena = hato::Hato::<dyn core::fmt::Debug>::default();
///
/// // Insert an element...
/// let x = arena.push(5_u8);
///
/// // ... then remove it
/// arena.remove(x);
///
/// // ! We can still use the handle to access it
/// assert_eq!(format!("{:?}", arena.get(x)), "5");
///
/// // Insert a new element into the arena
/// let _y = arena.push(9_u8);
///
/// // ! The old handle accesses the repurposed capacity
/// assert_eq!(format!("{:?}", arena.get(x)), "9");
/// ```
#[derive(Debug)]
pub struct Hato<Trait: ?Sized + Pointee<Metadata = DynMetadata<Trait>>>(Vec<Arena<Trait>>);

impl<Trait: ?Sized + Pointee<Metadata = DynMetadata<Trait>>> Default for Hato<Trait> {
    fn default() -> Self {
        Self(Vec::default())
    }
}

impl<Trait: ?Sized + Pointee<Metadata = DynMetadata<Trait>>> Clone for Hato<Trait> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<Trait: ?Sized + Pointee<Metadata = DynMetadata<Trait>>> Hato<Trait> {
    /// Insert `x` into the arena for its specific type.
    ///
    /// # Panics
    ///
    /// This function will panic if the number of arenas overflows the index type.
    #[inline]
    pub fn push<T: Unsize<Trait> + Unscrupulous>(&mut self, x: T) -> Handle {
        // Identify individual types at runtime using their virtual table pointer
        let vtable = get_metadata_of_ref(&x);

        // Index of arena that contains elements of type `T` and is not full
        let index_as_usize = self
            .0
            .iter()
            .position(|arena| arena.vtable == vtable && !arena.is_full())
            .unwrap_or_else(|| {
                // Create a new arena to store elements of type `T`
                self.0.push(Arena::new::<T>(vtable));

                // Point to arena that was just created
                self.0.len() - 1
            });

        // Bound the number of different types to limit the size of handles
        let index = u32::try_from(index_as_usize)
            .unwrap_or_else(|_| panic!("got more than `{}` arenas", u32::MAX));

        // Insert element into the arena
        let offset = self.0[index_as_usize].push(x);

        // Return handle for caller so they can access the element
        Handle { index, offset }
    }

    /// Retrieve the element identified by `handle` as a trait object.
    #[inline]
    #[must_use]
    pub fn get(&self, handle: Handle) -> &Trait {
        self.0[handle.index as usize].get(handle.offset)
    }

    /// Retrieve the element identified by `handle` as a mutable trait object.
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, handle: Handle) -> &mut Trait {
        self.0[handle.index as usize].get_mut(handle.offset)
    }

    /// Remove the element identified by `handle` from the collection.
    #[inline]
    pub fn remove(&mut self, handle: Handle) {
        self.0[handle.index as usize].remove(handle.offset);
    }
}

#[derive(Debug)]
struct Arena<Trait: ?Sized + Pointee<Metadata = DynMetadata<Trait>>> {
    vtable: DynMetadata<Trait>,
    bytes: AVec<u8>,
    slots: Vec<u32>,
}

impl<Trait: ?Sized + Pointee<Metadata = DynMetadata<Trait>>> Clone for Arena<Trait> {
    fn clone(&self) -> Self {
        Self {
            vtable: self.vtable,
            bytes: self.bytes.clone(),
            slots: self.slots.clone(),
        }
    }
}

impl<Trait: ?Sized + Pointee<Metadata = DynMetadata<Trait>>> Arena<Trait> {
    #[inline]
    fn new<T>(vtable: DynMetadata<Trait>) -> Self {
        // ! SAFETY: Force base pointer alignment so individual elements are always
        // ! stored at valid addresses, even on re-allocation events
        let bytes = AVec::new(align_of::<T>());

        Self {
            vtable,
            bytes,
            slots: Vec::new(),
        }
    }

    #[inline]
    fn is_full(&self) -> bool {
        u32::try_from(self.bytes.len()).is_err()
    }

    #[inline]
    fn push<T: Unsize<Trait> + Unscrupulous>(&mut self, x: T) -> u32 {
        // Check caller is inserting an element of the correct type
        debug_assert_eq!(self.vtable, get_metadata_of_ref(&x));

        // Reinterpret object as a slice of bytes to be copied to buffer
        let slice = as_slice_of_bytes(&x);

        // Position of the element in the buffer
        let offset = if let Some(offset) = self.slots.pop() {
            // Offset is a valid `usize` by initial construction in previous `push`
            #[allow(clippy::cast_possible_truncation)]
            let offset_as_usize = offset as usize;

            // Copy object over to buffer, overwriting previous element
            self.bytes[offset_as_usize..offset_as_usize + align_of::<T>()].copy_from_slice(slice);

            offset
        } else {
            // Fit byte offset in a `u32` to limit the size of handles
            let offset = u32::try_from(self.bytes.len())
                .expect("individual arenas should hold less than 4GB of data");

            // Copy object over to buffer, valid thanks to `Unscrupulous` trait bound
            self.bytes.extend_from_slice(slice);

            offset
        };

        // Prevent destructor from running on scope end
        core::mem::forget(x);

        offset
    }

    #[inline]
    fn get(&self, offset: u32) -> &Trait {
        unsafe {
            // ! SAFETY: Trait object points to a valid byte representation of this type
            &*from_raw_parts(self.bytes.as_ptr().add(offset as usize).cast(), self.vtable)
        }
    }

    #[inline]
    fn get_mut(&mut self, offset: u32) -> &mut Trait {
        unsafe {
            // ! SAFETY: Trait object points to a valid byte representation of this type
            let ptr = self.bytes.as_mut_ptr().add(offset as usize).cast();
            &mut *from_raw_parts_mut(ptr, self.vtable)
        }
    }

    #[inline]
    fn remove(&mut self, offset: u32) {
        self.slots.push(offset);
    }
}

/// Index to access an element stored in the arena.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Handle {
    index: u32,
    offset: u32,
}

/// Extract pointer to the virtual table of a specific type's implementation of `Trait`.
const fn get_metadata_of_ref<T, Trait>(ptr: &T) -> DynMetadata<Trait>
where
    T: Sized + Unscrupulous + Unsize<Trait>,
    Trait: ?Sized + Pointee<Metadata = DynMetadata<Trait>>,
{
    metadata(from_ref::<Trait>(ptr))
}
