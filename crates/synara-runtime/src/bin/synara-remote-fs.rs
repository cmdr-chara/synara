fn main() {
    if let Err(error) = synara_runtime::remote_fs_helper_main() {
        eprintln!("synara-remote-fs: {error}");
        std::process::exit(1);
    }
}
