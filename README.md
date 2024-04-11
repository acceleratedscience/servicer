### SERVICING: a small binary aimed at service configuration and cluster deployment for OPENAD

###### How to run this locally from source
 1. Clone this repository:

 ```bash
 git clone git@github.com:acceleratedscience/servicing.git
 ```
 2. Install Rust toolchain:
 ```bash
 curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
 ```
 3. Download Maturin (Python bindings for Rust)
 ```bash
 cargo install maturin
 ```
 4. Create Python virtual environment:
 ```bash
 virtualenv .venv
 ```
 5. Build the project:
 ```bash
 maturin develop
 ```
 6. Activate the virtual environment:
 ```bash
 source .venv/bin/activate
 ```
 7. Use Servicing
 ```bash
 python
 import servicing
 ```
