
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

To generate a Control Flow Graph (CFG) from a Rust source file, follow these steps:

1. Place the Rust file inside the `tests` folder (`src/tests/your_rust_file.rs`).
2. Run the project using Cargo:

```bash
   cargo run your_rust_file.rs
```

The CFG (Control Flow Graph) of your `your_rust_file.rs` will be generated and found inside `src/graphs/your_rust_file.dot`.

To view the CFG, you can use tools such as Graphviz, or you can use the online Graphviz viewer provided by Dreampuf:

[GraphvizOnline by Dreampuf](https://dreampuf.github.io/GraphvizOnline/)

## Acknowledgments

Generative AI has proven very helpful in several aspects, including tasks such as code formatting, error correction based on compiler feedback, and high-level conceptual reasoning. While it's important to approach these tools with caution and avoid blind reliance, they do offer additional perspectives and occasionally provide valuable guidance. 
It served as a valuable tool for speeding up code formatting and understanding errors.