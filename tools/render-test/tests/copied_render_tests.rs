// The render and query tests copied into metrics/ (the ones the globe and the
// sky add or change) on the Rust renderer, through manifests that differ from
// the shared suite's only in base_test_path.

mod suite;

#[test]
fn copied_render_tests() {
    assert_eq!(suite::run("copied_render_tests", "wgpu-copied", &[], Some(render_test::map::factory())), 0);
}
