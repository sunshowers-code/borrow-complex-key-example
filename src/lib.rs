// Copyright and related rights waived via CC0: https://creativecommons.org/publicdomain/zero/1.0/

//! An example for how to implement Borrow for complex keys.
//!
//! Thanks to Ivan Dubrov (http://idubrov.name/rust/2018/06/01/tricking-the-hashmap.html) for the
//! inspiration.

#![allow(unused_imports)]

use proptest::prelude::*;
use proptest_derive::Arbitrary;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[test]
fn basic() {
    // Consider a hash set of strings...
    let mut hash_set: HashSet<String> = HashSet::new();
    hash_set.insert("example-string".to_string());

    // Ordinarily, you need a &String to look up keys.
    let string_key: String = "example-string".to_string();
    assert!(hash_set.contains(&string_key));

    // But, it turns out you can also pass in a &str, not just a &String!
    let str_key: &str = "example-string";
    assert!(hash_set.contains(str_key));

    // How does this work? It's all based on the Borrow trait.
    // https://doc.rust-lang.org/std/borrow/trait.Borrow.html
    //
    // For an owned type O and a borrowed type B, O may implement Borrow<B> if:
    // - it's possible to implement a function borrow(&self): &B
    // - if implemented, Eq, Ord and Hash are *consistent* between O and B.
    //
    // String and str satisfy these conditions, so String implements Borrow<str>.
}

// But what about a user-defined type that's more complex than just a String? For example,
// consider this owned type:
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Arbitrary)]
struct OwnedKey {
    s: String,
    bytes: Vec<u8>,
}

// (You might have noticed the "Arbitrary" above. Put a pin in that.)

// ... and this borrowed type:
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct BorrowedKey<'a> {
    s: &'a str,
    bytes: &'a [u8],
}

#[test]
fn complex1() {
    // They're basically the same type, modulo ownership. Can we take a hash set of owned keys...
    let mut hash_set: HashSet<OwnedKey> = HashSet::new();
    hash_set.insert(OwnedKey {
        s: "foo".to_string(),
        bytes: b"abc".to_vec(),
    });

    // and use a borrowed key to look things up, thereby eliminating the need to allocate a new
    // owned key just for this?
    let _borrowed_key = BorrowedKey {
        s: "foo",
        bytes: b"abc",
    };
    // assert!(hash_set.contains(&_borrowed_key));
}

// It turns out that yes, we can do that through the power of trait objects! Here's how:
// (1) define a trait object that looks like this.
trait Key {
    fn key(&self) -> BorrowedKey;
}

// (2) Implement it for both the owned and borrowed versions.
impl Key for OwnedKey {
    fn key(&self) -> BorrowedKey {
        BorrowedKey {
            s: self.s.as_str(),
            bytes: self.bytes.as_slice(),
        }
    }
}

impl<'a> Key for BorrowedKey<'a> {
    fn key(&self) -> BorrowedKey {
        *self
    }
}

// For the rest of this example, we're going to make trait objects of type &(dyn Key + 'a)
// central to our strategy.
//
// OK, so...
//
// (3) Implement Borrow<dyn Key> for OwnedKey.
impl<'a> Borrow<dyn Key + 'a> for OwnedKey {
    fn borrow(&self) -> &(dyn Key + 'a) {
        // This is a simple coercion from the concrete type to a trait object.
        self
    }
}

// Now, remember that for Borrow to be valid, Eq, Hash and Ord need to be consistent. How do
// we ensure that? Let's see:

// (4) PartialEq and Eq turn out to be easy to do.
impl<'a> PartialEq for (dyn Key + 'a) {
    fn eq(&self, other: &Self) -> bool {
        // It's easy to see from the definition that the owned and borrowed types have a consistent
        // implementation. (Don't worry, we're actually going to verify this.)
        self.key().eq(&other.key())
    }
}

impl<'a> Eq for (dyn Key + 'a) {}

// (5) PartialOrd and Ord are similar.
//
// A couple of notes:
// - Importantly, this relies on the fact that the derive implementations for PartialOrd and Ord use
//   lexicographic ordering on struct member order.
// - You need to implement this if you're using a btree based data structure, not if you're only
//   using hash-based data structures.
impl<'a> PartialOrd for (dyn Key + 'a) {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key().partial_cmp(&other.key())
    }
}

impl<'a> Ord for (dyn Key + 'a) {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

// (6) Hash also turns out to be easy to do in this case, though in the general case getting a
// consistent impl may be trickier and may require implementing Hash by hand for the owned type.
//
// Implementing Hash is only necessary if you're using a hash-based data structure.
impl<'a> Hash for (dyn Key + 'a) {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state)
    }
}

// That's it! Now, we have everything we need to do this.
#[test]
fn complex2() {
    // They're basically the same type, modulo ownership. Can we take a hash set of owned keys...
    let mut hash_set: HashSet<OwnedKey> = HashSet::new();
    hash_set.insert(OwnedKey {
        s: "foo".to_string(),
        bytes: b"abc".to_vec(),
    });

    // and use a borrowed key to look things up, thereby eliminating the need to allocate a new
    // owned key just for this?
    let borrowed_key = BorrowedKey {
        s: "foo",
        bytes: b"abc",
    };
    // Note the coercion into a trait object.
    assert!(hash_set.contains(&borrowed_key as &dyn Key));
}

// ... not so fast, though! We've attempted to satisfy the constraints required for the Borrow impl.
// We've got to test them.
//
// The constraints describe *properties* that must be satisfied. The best way to ensure they are is
// to use property-based testing.
//
// There's much more to property-based testing than we can cover here, but
// https://blog.jessitron.com/2013/04/25/property-based-testing-what-is-it/ is a good intro.
//
// We're going to use the proptest framework to write our property-based tests.
proptest! {
    // Here's where that Arbitrary above is useful. It's a simple way to generate random values of
    // your structure.
    #[test]
    fn consistent_borrow(owned1 in any::<OwnedKey>(), owned2 in any::<OwnedKey>()) {
        // owned1 and owned2 will be populated with random values of OwnedKey. That's enough for us
        // to start testing various properties.
        //
        // Reminder that what we want is for the *owned* and *borrowed* impls to be consistent.
        // owned1 and owned2 are the owned keys. The borrowed impls are:
        let borrowed1: &dyn Key = &owned1;
        let borrowed2: &dyn Key = &owned2;

        // Awesome! That's all the setup we need. Time to test all of this. First, equality:
        assert_eq!(owned1 == owned2, borrowed1 == borrowed2, "consistent Eq");

        // PartialOrd and Ord:
        assert_eq!(owned1.partial_cmp(&owned2), borrowed1.partial_cmp(borrowed2), "consistent PartialOrd");
        assert_eq!(owned1.cmp(&owned2), borrowed1.cmp(borrowed2), "consistent Ord");

        // And finally, Hash. This requires a tiny bit of setup.
        fn hash_output(x: impl Hash) -> u64 {
            let mut hasher = DefaultHasher::new();
            x.hash(&mut hasher);
            hasher.finish()
        }

        assert_eq!(hash_output(&owned1), hash_output(&borrowed1), "consistent Hash")
        assert_eq!(hash_output(&owned2), hash_output(&borrowed2), "consistent Hash");

        // and that's it! Any implementation that satisfies these properties is a valid
        // Borrow implementation. A property-based test guarantees that with high confidence.
        //
        // Here's some stuff to play around with:
        // (1) does this work for enums as well?
        // (2) try swapping the order of fields in either OwnedKey or BorrowedKey, and see what
        //     happens to this property test.
    }
}
