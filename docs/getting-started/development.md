# Development
If you want to be part of the SciWIn Client development feel free to checkout our Repository: [SciWIn Client](https://github.com/fairagro/m4.4_sciwin_client).

## Building SciWIn Client
To build SciWIn Client locally the repository needs to be cloned from GitHub:

### Cloning the repository
```
# Clone the repository
git clone https://github.com/fairagro/m4.4_sciwin_client

# Navigate to the project directory
cd m4.4_sciwin_client
```
Furthermore a [Rust](https://www.rust-lang.org/) environment needs to be set up.

### Setting up Rust
=== ":simple-linux: Linux"
    Use the following Bash command to install Rustup
    ```
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

=== ":fontawesome-brands-windows: Windows"
    Use the Rustup installer from the [Rust website](https://www.rust-lang.org/tools/install).

    [RUSTUP-INIT.EXE (32-BIT)](https://static.rust-lang.org/rustup/dist/i686-pc-windows-msvc/rustup-init.exe){ .md-button .md-button--primary }
    [RUSTUP-INIT.EXE (64-BIT)](https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe){ .md-button .md-button--primary }

    For WSL2 the following command can be used.
    ```
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

### Build and test
If VSCode is used as development environment the [rust-analyzer](https://code.visualstudio.com/docs/languages/rust) extension needs to be installed.

SciWIn Client can than be build using the  `cargo` commandline tool
```
cargo build
```

To run & build `cargo run` can be used followed by the commands and parameters of SciWIn Client, e.g. `cargo run tool create python echo.py`.

For linting clippy is used which can be called using `cargo clippy --all-targets`

Unit and integration tests can be run using `cargo test`. If logging to stdout needs to be displayed the no-capture flag needs to be set `cargo test -- --nocapture`