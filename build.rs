fn main() {
    let home = std::env::var("HOME").expect("${HOME} is missing");
    println!("cargo:rustc-env=LD_LIBRARY_PATH={home}/.pyenv/versions/3.10.14/lib");
}
