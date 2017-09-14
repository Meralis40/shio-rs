// Extracted code from https://github.com/reem/rust-typemap, and adapted for shio usage
use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::hash::{BuildHasherDefault, Hasher};
use std::ptr;

use unsafe_any::{UnsafeAny, UnsafeAnyExt};

#[derive(Default)]
pub struct TypeIdHasherValue {
    value: u64,
}

impl Hasher for TypeIdHasherValue {
    fn finish(&self) -> u64 {
        self.value
    }

    fn write(&mut self, bytes: &[u8]) {
        if bytes.len() != 8 {
            panic!("unexpected len for typeid hash");
        }

        let buffer = &mut self.value as *mut u64;
        let buffer = buffer as *mut u8;

        let orig = bytes.as_ptr();

        unsafe {
            ptr::copy_nonoverlapping(orig, buffer, 8);
        }
    }
}

// exported only for avoid "private in public" error
#[doc(hidden)]
pub unsafe trait Implements<A: ?Sized + UnsafeAnyExt> {
    fn into_object(self) -> Box<A>;
}

unsafe impl<T: UnsafeAny> Implements<UnsafeAny> for T {
    fn into_object(self) -> Box<UnsafeAny> {
        Box::new(self)
    }
}

unsafe impl<T: UnsafeAny + Send + Sync> Implements<(UnsafeAny + Send + Sync)> for T {
    fn into_object(self) -> Box<UnsafeAny + Send + Sync> {
        Box::new(self)
    }
}

#[derive(Default, Debug)]
pub struct TypeMap<A: ?Sized = UnsafeAny>
where
    A: UnsafeAnyExt,
{
    data: HashMap<TypeId, Box<A>, BuildHasherDefault<TypeIdHasherValue>>,
}

/// This trait defines the relationship between keys and values in a `TypeMap`.
///
/// It is implemented for Keys, with a phantom associated type for the values.
pub trait Key: Any {
    /// The value type associated with this key type.
    type Value: Any;
}

#[cfg(feature = "nightly")]
default impl<T: 'static> Key for T {
    type Value = T;
}

impl TypeMap {
    /// Create a new, empty TypeMap.
    pub fn new() -> TypeMap {
        TypeMap::custom()
    }
}

impl<A: UnsafeAnyExt + ?Sized> TypeMap<A> {
    /// Create a new, empty TypeMap.
    ///
    /// Can be used with any `A` parameter; `new` is specialized to get around
    /// the required type annotations when using this function.
    pub fn custom() -> TypeMap<A> {
        TypeMap {
            data: HashMap::default(),
        }
    }

    /// Insert a value into the map with a specified key type.
    pub fn insert<K: Key>(&mut self, val: K::Value) -> Option<K::Value>
    where
        K::Value: Any + Implements<A>,
    {
        self.data
            .insert(TypeId::of::<K>(), val.into_object())
            .map(|v| unsafe { *v.downcast_unchecked::<K::Value>() })
    }

    /// Find a value in the map and get a reference to it.
    pub fn get<K: Key>(&self) -> Option<&K::Value>
    where
        K::Value: Any + Implements<A>,
    {
        self.data
            .get(&TypeId::of::<K>())
            .map(|v| unsafe { v.downcast_ref_unchecked::<K::Value>() })
    }
}

pub type ShareMap = TypeMap<UnsafeAny + Sync + Send>;
