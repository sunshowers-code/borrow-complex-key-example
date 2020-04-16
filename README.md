# Example: implementing Borrow for complex keys

This repository contains a working Rust example for how to implement Borrow for non-trivial keys, written in a literate
programming style.

Given, for example:

```rust
struct OwnedKey {
    s: String,
    bytes: Vec<u8>,
}

struct BorrowedKey<'a> {
    s: &'a str,
    bytes: &'a [u8],
}
```

how can you use `BorrowedKey` instances to do lookups for a
`HashSet<OwnedKey>` or `BTreeSet<OwnedKey>`?

Head on over to [`src/lib.rs`](src/lib.rs) to find out!

## License

CC0: https://creativecommons.org/publicdomain/zero/1.0/
