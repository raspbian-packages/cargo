// Copyright 2013 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Dictionaries of key-value pairs.

pub use core_foundation_sys::dictionary::*;

use core_foundation_sys::base::{CFTypeRef, CFRelease, kCFAllocatorDefault};
use std::mem;
use std::os::raw::c_void;
use std::ptr;
use std::marker::PhantomData;

use base::{ItemRef, FromVoid, ToVoid};
use base::{CFIndexConvertible, TCFType};

// consume the type parameters with PhantomDatas
pub struct CFDictionary<K = *const c_void, V = *const c_void>(CFDictionaryRef, PhantomData<K>, PhantomData<V>);

impl<K, V> Drop for CFDictionary<K, V> {
    fn drop(&mut self) {
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

impl_TCFType!(CFDictionary<K, V>, CFDictionaryRef, CFDictionaryGetTypeID);
impl_CFTypeDescription!(CFDictionary);

impl<K, V> CFDictionary<K, V> {
    pub fn from_CFType_pairs(pairs: &[(K, V)]) -> CFDictionary<K, V> where K: TCFType, V: TCFType {
        let (keys, values): (Vec<CFTypeRef>, Vec<CFTypeRef>) = pairs
            .iter()
            .map(|&(ref key, ref value)| (key.as_CFTypeRef(), value.as_CFTypeRef()))
            .unzip();

        unsafe {
            let dictionary_ref = CFDictionaryCreate(kCFAllocatorDefault,
                                                    mem::transmute(keys.as_ptr()),
                                                    mem::transmute(values.as_ptr()),
                                                    keys.len().to_CFIndex(),
                                                    &kCFTypeDictionaryKeyCallBacks,
                                                    &kCFTypeDictionaryValueCallBacks);
            TCFType::wrap_under_create_rule(dictionary_ref)
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe {
            CFDictionaryGetCount(self.0) as usize
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn contains_key(&self, key: &K) -> bool where K: ToVoid<K> {
        unsafe { CFDictionaryContainsKey(self.0, key.to_void()) != 0 }
    }

    #[inline]
    pub fn find<'a, T: ToVoid<K>>(&'a self, key: T) -> Option<ItemRef<'a, V>> where V: FromVoid, K: ToVoid<K> {
        unsafe {
            let mut value: *const c_void = ptr::null();
            if CFDictionaryGetValueIfPresent(self.0, key.to_void(), &mut value) != 0 {
                Some(V::from_void(value))
            } else {
                None
            }
        }
    }

    /// # Panics
    ///
    /// Panics if the key is not present in the dictionary. Use `find` to get an `Option` instead
    /// of panicking.
    #[inline]
    pub fn get<'a, T: ToVoid<K>>(&'a self, key: T) -> ItemRef<'a, V> where V: FromVoid, K: ToVoid<K> {
        let ptr = key.to_void();
        self.find(key).expect(&format!("No entry found for key {:p}", ptr))
    }

    pub fn get_keys_and_values(&self) -> (Vec<*const c_void>, Vec<*const c_void>) {
        let length = self.len();
        let mut keys = Vec::with_capacity(length);
        let mut values = Vec::with_capacity(length);

        unsafe {
            CFDictionaryGetKeysAndValues(self.0, keys.as_mut_ptr(), values.as_mut_ptr());
            keys.set_len(length);
            values.set_len(length);
        }

        (keys, values)
    }
}

// consume the type parameters with PhantomDatas
pub struct CFMutableDictionary<K = *const c_void, V = *const c_void>(CFMutableDictionaryRef, PhantomData<K>, PhantomData<V>);

impl<K, V> Drop for CFMutableDictionary<K, V> {
    fn drop(&mut self) {
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

impl_TCFType!(CFMutableDictionary<K, V>, CFMutableDictionaryRef, CFDictionaryGetTypeID);
impl_CFTypeDescription!(CFMutableDictionary);

impl<K, V> CFMutableDictionary<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: isize) -> Self {
        unsafe {
            let dictionary_ref = CFDictionaryCreateMutable(kCFAllocatorDefault,
                                                           capacity as _,
                                                           &kCFTypeDictionaryKeyCallBacks,
                                                           &kCFTypeDictionaryValueCallBacks);
            TCFType::wrap_under_create_rule(dictionary_ref)
        }
    }

    pub fn copy_with_capacity(&self, capacity: isize) -> Self {
        unsafe {
            let dictionary_ref = CFDictionaryCreateMutableCopy(kCFAllocatorDefault, capacity as _, self.0);
            TCFType::wrap_under_get_rule(dictionary_ref)
        }
    }

    pub fn from_CFType_pairs(pairs: &[(K, V)]) -> CFMutableDictionary<K, V> where K: ToVoid<K>, V: ToVoid<V> {
        let mut result = Self::with_capacity(pairs.len() as _);
        for &(ref key, ref value) in pairs {
            result.add(key, value);
        }
        result
    }

    // Immutable interface

    #[inline]
    pub fn len(&self) -> usize {
        unsafe {
            CFDictionaryGetCount(self.0) as usize
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn contains_key(&self, key: *const c_void) -> bool {
        unsafe {
            CFDictionaryContainsKey(self.0, key) != 0
        }
    }

    #[inline]
    pub fn find<'a>(&'a self, key: &K) -> Option<ItemRef<'a, V>> where V: FromVoid, K: ToVoid<K> {
        unsafe {
            let mut value: *const c_void = ptr::null();
            if CFDictionaryGetValueIfPresent(self.0, key.to_void(), &mut value) != 0 {
                Some(V::from_void(value))
            } else {
                None
            }
        }
    }

    /// # Panics
    ///
    /// Panics if the key is not present in the dictionary. Use `find` to get an `Option` instead
    /// of panicking.
    #[inline]
    pub fn get<'a>(&'a self, key: &K) -> ItemRef<'a, V> where V: FromVoid, K: ToVoid<K> {
        let ptr = key.to_void();
        self.find(&key).expect(&format!("No entry found for key {:p}", ptr))
    }

    pub fn get_keys_and_values(&self) -> (Vec<*const c_void>, Vec<*const c_void>) {
        let length = self.len();
        let mut keys = Vec::with_capacity(length);
        let mut values = Vec::with_capacity(length);

        unsafe {
            CFDictionaryGetKeysAndValues(self.0, keys.as_mut_ptr(), values.as_mut_ptr());
            keys.set_len(length);
            values.set_len(length);
        }

        (keys, values)
    }

    // Mutable interface

    /// Adds the key-value pair to the dictionary if no such key already exist.
    #[inline]
    pub fn add(&mut self, key: &K, value: &V) where K: ToVoid<K>, V: ToVoid<V> {
        unsafe { CFDictionaryAddValue(self.0, key.to_void(), value.to_void()) }
    }

    /// Sets the value of the key in the dictionary.
    #[inline]
    pub fn set(&mut self, key: K, value: V) where K: ToVoid<K>, V: ToVoid<V> {
        unsafe { CFDictionarySetValue(self.0, key.to_void(), value.to_void()) }
    }

    /// Replaces the value of the key in the dictionary.
    #[inline]
    pub fn replace(&mut self, key: K, value: V) where K: ToVoid<K>, V: ToVoid<V> {
        unsafe { CFDictionaryReplaceValue(self.0, key.to_void(), value.to_void()) }
    }

    /// Removes the value of the key from the dictionary.
    #[inline]
    pub fn remove(&mut self, key: K) where K: ToVoid<K> {
        unsafe { CFDictionaryRemoveValue(self.0, key.to_void()) }
    }

    #[inline]
    pub fn remove_all(&mut self) {
        unsafe { CFDictionaryRemoveAllValues(self.0) }
    }
}


#[cfg(test)]
pub mod test {
    use super::*;
    use base::{CFType, TCFType};
    use boolean::CFBoolean;
    use number::CFNumber;
    use string::CFString;


    #[test]
    fn dictionary() {
        let bar = CFString::from_static_string("Bar");
        let baz = CFString::from_static_string("Baz");
        let boo = CFString::from_static_string("Boo");
        let foo = CFString::from_static_string("Foo");
        let tru = CFBoolean::true_value();
        let n42 = CFNumber::from(42);

        let d = CFDictionary::from_CFType_pairs(&[
            (bar.as_CFType(), boo.as_CFType()),
            (baz.as_CFType(), tru.as_CFType()),
            (foo.as_CFType(), n42.as_CFType()),
        ]);

        let (v1, v2) = d.get_keys_and_values();
        assert!(v1 == &[bar.as_CFTypeRef(), baz.as_CFTypeRef(), foo.as_CFTypeRef()]);
        assert!(v2 == &[boo.as_CFTypeRef(), tru.as_CFTypeRef(), n42.as_CFTypeRef()]);
    }

    #[test]
    fn mutable_dictionary() {
        let bar = CFString::from_static_string("Bar");
        let baz = CFString::from_static_string("Baz");
        let boo = CFString::from_static_string("Boo");
        let foo = CFString::from_static_string("Foo");
        let tru = CFBoolean::true_value();
        let n42 = CFNumber::from(42);

        let mut d = CFMutableDictionary::<CFString, CFType>::new();
        d.add(&bar, &boo.as_CFType());
        d.add(&baz, &tru.as_CFType());
        d.add(&foo, &n42.as_CFType());
        assert_eq!(d.len(), 3);

        let (v1, v2) = d.get_keys_and_values();
        assert!(v1 == &[bar.as_CFTypeRef(), baz.as_CFTypeRef(), foo.as_CFTypeRef()]);
        assert!(v2 == &[boo.as_CFTypeRef(), tru.as_CFTypeRef(), n42.as_CFTypeRef()]);

        d.remove(baz);
        assert_eq!(d.len(), 2);

        let (v1, v2) = d.get_keys_and_values();
        assert!(v1 == &[bar.as_CFTypeRef(), foo.as_CFTypeRef()]);
        assert!(v2 == &[boo.as_CFTypeRef(), n42.as_CFTypeRef()]);

        d.remove_all();
        assert_eq!(d.len(), 0)
    }

    #[test]
    fn dict_find_and_contains_key() {
        let dict = CFDictionary::from_CFType_pairs(&[
            (
                CFString::from_static_string("hello"),
                CFBoolean::true_value(),
            ),
        ]);
        let key = CFString::from_static_string("hello");
        let invalid_key = CFString::from_static_string("foobar");

        assert!(dict.contains_key(&key));
        assert!(!dict.contains_key(&invalid_key));

        let value = dict.find(&key).unwrap().clone();
        assert_eq!(value, CFBoolean::true_value());
        assert_eq!(dict.find(&invalid_key), None);
    }
}
