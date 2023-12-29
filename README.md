
# Secured Rust

## Introduction

Secured Rust is a project aimed at enhancing the reliability and security of Rust code through Weakest Precondition (WP) calculus. The project focuses on verifying preconditions, postconditions, and invariants to ensure that Rust code behaves as intended. This approach aids in identifying and rectifying potential vulnerabilities or logic errors in Rust applications.

## Project Goals

By employing WP calculus, the project aims to ensure that the Rust code adheres to its specified behavior.
Verifying preconditions, postconditions, and invariants contribute to making the codebase more secure and less prone to exploits.

## Getting Started

### Prerequisites

- Rust Programming Language: Ensure you have [Rust installed](https://www.rust-lang.org/tools/install).
- Cargo: Comes bundled with Rust.
- Additional dependencies: Listed in `Cargo.toml`.

### Installation

1. **Clone the Repository**:
    ```bash
    git clone https://github.com/vasilevlaicu/secured-rust.git
    cd secured-rust
    ```

2. **Install Dependencies**:
    ```bash
    cargo build
    ```

### Running the Project

- To generate a Control Flow Graph (CFG) from a Rust source file:
    ```bash
    cargo run -- path/to/your_rust_file.rs
    ```
