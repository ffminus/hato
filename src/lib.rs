// Use `README.md` as documentation home page, to reduce duplication
#![doc = include_str!("../README.md")]

// Re-export dependencies to be used in macro invocations
pub use aligned_vec;
pub use unscrupulous;

/// Declare a new type to store trait objects in vectors grouped by type.
///
/// See [crate documentation][`hato`] for more details.
#[macro_export]
macro_rules! hato {
    ($interface:path, $arena:ident, $handle:ident, with_aba = true) => {
        $crate::hato!($interface, $arena, $handle, with_aba = true, vis = pub(self));
    };

    ($interface:path, $arena:ident, $handle:ident, with_aba = true, mod = $mod:ident) => {
        $crate::hato!($interface, $arena, $handle, with_aba = true, vis = pub(self), mod = $mod);
    };

    ($interface:path, $arena:ident, $handle:ident, with_aba = true, vis = $visibility:vis) => {
        $crate::hato!($interface, $arena, $handle, with_aba = true, vis = $visibility, mod = _hato_mod);
    };

    ($interface:path, $arena:ident, $handle:ident, with_aba = true, vis = $visibility:vis, mod = $mod:ident) => {
        $crate::hato!($interface, $arena, $handle, with_aba = true, vis = $visibility, mod = $mod, u32, u32);
    };

    ($interface:path, $arena:ident, $handle:ident, with_aba = true, vis = $visibility:vis, mod = $mod:ident, $index:ty, $offset:ty) => {
        paste::paste! {
            $crate::hato!($interface, [<$mod _trait>], $arena, $handle, with_aba = true, vis = $visibility, mod = $mod, $index, $offset);
        }
    };

    ($interface:path, $interface_alias:ident, $arena:ident, $handle:ident, with_aba = true, vis = $visibility:vis, mod = $mod:ident, $index:ty, $offset:ty) => {
        // Pull declared types into scope with names provided by caller
        #[allow(unused_imports, clippy::needless_pub_self)]
        $visibility use $mod::Arenas as $arena;
        #[allow(unused_imports, clippy::needless_pub_self)]
        $visibility use $mod::Handle as $handle;

        // Alias provided path to trait to be imported in nested module
        use $interface as $interface_alias;

        /// Private module to reduce pollution in calling namespace
        mod $mod {
            use core::mem::{align_of, transmute};

            use $crate::aligned_vec::AVec;
            use $crate::unscrupulous::{as_slice_of_bytes, Unscrupulous};

            use super::$interface_alias as Interface;

            /// Arena of heterogeneous trait objects, stored by type in separate vectors.
            ///
            /// See documentation of [`hato::hato`] for more details.
            #[derive(Clone, Default)]
            pub struct Arenas(Vec<Arena>);

            /// Pull pointer to virtual table for a specific type's implementation of `Interface`.
            fn extract_vtable_pointer(x: &impl Interface) -> *const u8 {
                // ! SAFETY: Split fat pointer, event though layout stability is not guaranteed
                unsafe { transmute::<&dyn Interface, [_; 2]>(x)[1] }
            }

            impl Arenas {
                /// Insert `x` into the arena for its specific type.
                ///
                /// # Panics
                ///
                /// This function will panic if the number of arenas overflows the index type.
                #[inline]
                pub fn push<T: Interface + Unscrupulous>(&mut self, x: T) -> Handle {
                    // Identify individual types at runtime using their virtual table pointer
                    let vtable = extract_vtable_pointer(&x);

                    // Index of arena that contains elements of type `T` and is not full
                    let index_as_usize = self
                        .0
                        .iter()
                        .position(|arena| arena.vtable == vtable && arena.is_not_full())
                        .unwrap_or_else(|| {
                            // Create a new arena to store elements of type `T`
                            self.0.push(Arena::new::<T>(vtable));

                            // Point to arena that was just created
                            self.0.len() - 1
                        });

                    // Bound the number of different types to limit the size of handles
                    let index = <$index>::try_from(index_as_usize)
                        .unwrap_or_else(|_| panic!("got more than `{}` arenas", <$index>::MAX));

                    // Insert element in arena
                    let offset = self.0[index_as_usize].push(x);

                    // Return handle for caller so they can access the element
                    Handle { index, offset }
                }

                /// Retrieve the element identified by `handle` as a trait object.
                #[allow(dead_code)]
                #[inline]
                pub fn get(&self, handle: Handle) -> &dyn Interface {
                    #[allow(clippy::cast_possible_truncation)]
                    self.0[handle.index as usize].get(handle.offset)
                }

                /// Retrieve the element identified by `handle` as a mutable trait object.
                #[allow(dead_code)]
                #[inline]
                pub fn get_mut(&mut self, handle: Handle) -> &mut dyn Interface {
                    #[allow(clippy::cast_possible_truncation)]
                    self.0[handle.index as usize].get_mut(handle.offset)
                }

                /// Remove the element identified by `handle` from the collection.
                #[allow(dead_code)]
                #[inline]
                pub fn remove(&mut self, handle: Handle) {
                    #[allow(clippy::cast_possible_truncation)]
                    self.0[handle.index as usize].remove(handle.offset);
                }
            }

            #[derive(Clone)]
            struct Arena {
                vtable: *const u8,
                bytes: AVec<u8>,
                slots: Vec<$offset>,
            }

            impl Arena {
                #[inline]
                fn new<T>(vtable: *const u8) -> Self {
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
                fn is_not_full(&self) -> bool {
                    u32::try_from(self.bytes.len()).is_ok()
                }

                #[inline]
                fn push<T: Interface + Unscrupulous>(&mut self, x: T) -> $offset {
                    // Check caller is inserting an element of the correct type
                    debug_assert_eq!(extract_vtable_pointer(&x), self.vtable);

                    // Reinterpret object as a slice of bytes to be copied to buffer
                    let slice = as_slice_of_bytes(&x);

                    // Position of the element in the buffer
                    let offset = if let Some(offset) = self.slots.pop() {
                        // Offset is a valid `usize` by initial construction in previous `push`
                        #[allow(clippy::cast_possible_truncation)]
                        let offset_as_usize = offset as usize;

                        // Copy object over to buffer, overwriting previous element
                        self.bytes[offset_as_usize..offset_as_usize + align_of::<T>()]
                            .copy_from_slice(slice);

                        offset
                    } else {
                        // Fit byte offset in a `u32` to limit size of handles
                        let offset = <$offset>::try_from(self.bytes.len())
                            .expect("individual arenas to hold less than 4GB of data");

                        // TODO: Replace with `extend_from_slice` from `aligned-vec` version 0.6.0
                        // Copy object over to buffer, valid thanks to `Unscrupulous` trait bound
                        for byte in slice {
                            self.bytes.push(*byte);
                        }

                        offset
                    };

                    // Prevent destructor from running on scope end
                    core::mem::forget(x);

                    offset
                }

                #[inline]
                fn get(&self, offset: $offset) -> &dyn Interface {
                    // ! SAFETY: Trait object points to a valid byte representation of this type
                    unsafe { transmute(self.get_as_array_of_pointers(offset)) }
                }

                #[inline]
                fn get_mut(&mut self, offset: $offset) -> &mut dyn Interface {
                    // ! SAFETY: Trait object points to a valid byte representation of this type
                    unsafe { transmute(self.get_as_array_of_pointers(offset)) }
                }

                #[inline]
                fn get_as_array_of_pointers(&self, offset: $offset) -> [*const u8; 2] {
                    // Construct pointer to trait object data
                    #[allow(clippy::cast_possible_truncation)]
                    let ptr = self.bytes.as_ptr().wrapping_add(offset as usize);

                    // Mimick memory layout of trait objects with an array of pointers
                    [ptr, self.vtable]
                }

                #[inline]
                fn remove(&mut self, offset: $offset) {
                    self.slots.push(offset);
                }
            }

            /// Index to access an element stored in the arena.
            #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
            pub struct Handle {
                index: $index,
                offset: $offset,
            }
        }
    };
}
