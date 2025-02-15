
# Memcached

This is a custom implementation of a Memcached-compatible server written in Rust. It supports essential Memcached commands and is designed for efficient in-memory key-value storage with concurrent client handling.

## Features

- **Core Commands:** Implements essential Memcached commands (`set`, `get`, `add`, `replace`, `append`, `prepend`) for effective key-value data storage and retrieval.
- **Key Expiration:** Supports configurable expiration times for cached items, ensuring automatic invalidation of stale data.
- **Concurrent Connections:** Utilizes Rust's multithreading to manage multiple client connections simultaneously, maintaining high performance and thread safety.


## Getting Started

### Prerequisites

[Rust](https://www.rust-lang.org/) programming language installed

### Installation

1. Clone the repository:
```bash
git clone https://github.com/Dhanush-arch/memcached-rs.git
```
2. Navigate to the project directory:
```bash
cd memcached-rs
```
3. Build the project:
```bash
cargo build --release
```

## Usage

Run the server with the default port (11211):

```bash
./target/release/memcached
```
Or specify a custom port:
```bash
./target/release/memcached -p <port_number>
```
You can interact with the server using a Telnet client:
```bash
telnet localhost 11211
```

## Example

```bash
# Set a value
set mykey 0 60 5
hello
STORED

# Get the value
get mykey
VALUE hello 0 5
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.