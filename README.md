# sqlite3-inet
A rusty SQLite3 extension providing MAC ~~and IP address~~ (WIP) utilities. Theoretically supporting any platform that Rust + SQLite3 can compile to, with a single command.

Inspired by the existing, C-based (`sqlite3-inet` project ran by mobigroup)[https://github.com/mobigroup/sqlite3-inet].

# Build Instructions
Set two environment variables:
`SQLITE3_INCLUDE_DIR=sqlite3`
`SQLITE3_LIB_DIR=sqlite3`

Run `cargo build`. Release build recommended when building the final library for SQLite's use. Debug mode has some significant performance penaltys.
