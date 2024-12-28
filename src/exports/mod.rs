
/// A collection of SQLite functions for dealing with MAC addresses, and their associated vendor affiliations (OUIs).
///
/// Each function accepts MAC addresses in varying formats (though only the first is shown in example usages for brevity)
/// * `aa-bb-cc-dd-ee-ff`
/// * `aa:bb:cc:dd:ee:ff`
/// * `aabb.ccdd.eeff`
/// * `aabbccddeeff`
/// * `0xaabbccddeeff`
///
/// See the [MAC_FORMAT](crate::exports::mac::format) function to convert MAC addresses between known formats.
pub mod mac;

/// some documentation
pub mod inet;

// figure out a way to generate SQL tests in build.rs from rustdoc examples, and include! them here?
