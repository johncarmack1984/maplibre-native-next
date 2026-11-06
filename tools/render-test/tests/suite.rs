// Runs the runner as src/main.rs would: from the package directory (where
// `cargo test` runs the tests), with the manifest for the OS (macOS adds the
// C++ WebGPU ignore list, since wgpu renders through Metal there), and the
// HTML report and the actual/diff images in target/tmp/<name>.

use std::path::Path;

use render_test::runner::ImplFactory;

/// The runner's exit code for `args` (after `--manifestPath` and `--output`),
/// with `manifests/<os>-<manifest>.json`.
pub fn run(name: &str, manifest: &str, args: &[&str], factory: Option<ImplFactory>) -> i32 {
    let os = if cfg!(target_os = "macos") { "macos" } else { "linux" };
    let manifest = format!("manifests/{os}-{manifest}.json");
    let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let mut argv = vec!["render-test", "--manifestPath", &manifest, "--output", output.to_str().unwrap()];
    argv.extend(args);
    render_test::run_render_tests(&argv.iter().map(|arg| arg.to_string()).collect::<Vec<_>>(), factory)
}
