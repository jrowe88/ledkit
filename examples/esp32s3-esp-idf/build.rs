fn main() {
    // Passes ESP-IDF build settings (linker args, cfgs) from esp-idf-sys to this crate.
    embuild::espidf::sysenv::output();
}
