fn main() {
    // Export `imruski_init` by name so the manual-map injector can find
    // it in the PE export table and call it as:  imruski_init(remote_base)
    println!("cargo:rustc-link-arg=/EXPORT:imruski_init");
}

