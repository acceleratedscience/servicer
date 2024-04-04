<img src="https://github.com/acceleratedscience/stressor/assets/14236737/3b48d7f4-4605-406d-95b3-062e9e694b2c" width="100"></br>
### SERVICING: a small binary aimed at service configuration and cluster deployment for OPENAD

###### How to run this locally...
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
 4. Build the project:
 ```bash
 maturin develop
 ```
 5. Create Python virtual environment:
 ```bash
 virtualenv .venv
 ```
 6. Activate the virtual environment:
 ```bash
 source .venv/bin/activate
 ```
