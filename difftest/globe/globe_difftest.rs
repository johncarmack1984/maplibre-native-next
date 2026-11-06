// Difftest for the globe camera: TransformState, Transform and
// VerticalPerspectiveProjection (src/mln/map/transform_state.cpp,
// transform.cpp, vertical_perspective_projection.cpp) against the C++ core,
// over random cameras from Mercator to the globe, the poles, the antimeridian
// and zooms below the minimum included. Each case jumps a Transform to the
// camera on both sides and compares, bit for bit, what the jump settles on,
// the globe's view-projection matrix, clipping plane and camera position, and
// the mapping between screen and LatLng both ways.

use mln_map::transform::{NullTransformObserver, Transform};
use mln_map_types::{CameraOptions, ConstrainMode, ViewportMode};
use mln_style_types::{ProjectionDefinition, ProjectionType};
use mln_util::geo::{LatLng, ScreenCoordinate, WrapMode};
use mln_util::size::Size;
use proptest::prelude::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct GlobeCamera {
    width: u32,
    height: u32,
    lat: f64,
    lng: f64,
    zoom: f64,
    bearing: f64,
    pitch: f64,
    transition: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct GlobeState {
    lat: f64,
    lng: f64,
    zoom: f64,
    bearing: f64,
    pitch: f64,
    view_projection: [f64; 16],
    clipping_plane: [f64; 4],
    camera_position: [f64; 3],
    globe: u8,
}

unsafe extern "C" {
    fn ref_globe_jump(camera: *const GlobeCamera, out: *mut GlobeState) -> i32;
    fn ref_globe_screen_to_lat_lng(camera: *const GlobeCamera, x: f64, y: f64, lat: *mut f64, lng: *mut f64) -> i32;
    fn ref_globe_lat_lng_to_screen(
        camera: *const GlobeCamera,
        lat: f64,
        lng: f64,
        x: *mut f64,
        y: *mut f64,
        occluded: *mut u8,
    ) -> i32;
}

fn set_up(camera: &GlobeCamera) -> Transform {
    let mut transform =
        Transform::new(Rc::new(NullTransformObserver), ConstrainMode::HeightOnly, ViewportMode::Default);
    transform.resize(Size::new(camera.width, camera.height)).expect("size");
    transform.set_projection_definition(&ProjectionDefinition::between(
        ProjectionType::Mercator,
        ProjectionType::VerticalPerspective,
        camera.transition,
    ));
    transform.jump_to(
        &CameraOptions::default()
            .with_center(Some(LatLng::new(camera.lat, camera.lng).expect("LatLng")))
            .with_zoom(Some(camera.zoom))
            .with_bearing(Some(camera.bearing))
            .with_pitch(Some(camera.pitch)),
    );
    transform
}

/// The same double, bit for bit (any NaN matches any NaN).
fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

fn same_all(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(*x, *y))
}

/// The Rust side of `ref_globe_jump`; `None` where the C++ would throw.
fn rust_jump(camera: &GlobeCamera) -> Option<GlobeState> {
    catch_unwind(AssertUnwindSafe(|| {
        let transform = set_up(camera);
        let state = transform.get_state();
        let center = state.get_lat_lng(WrapMode::Unwrapped);
        let globe = state.is_globe_rendering();
        GlobeState {
            lat: center.latitude(),
            lng: center.longitude(),
            zoom: state.get_zoom(),
            bearing: state.get_bearing(),
            pitch: state.get_pitch(),
            view_projection: if globe { state.get_globe_view_projection_matrix() } else { [0.0; 16] },
            clipping_plane: if globe { state.get_globe_clipping_plane() } else { [0.0; 4] },
            camera_position: if globe { state.get_globe_camera_position() } else { [0.0; 3] },
            globe: u8::from(globe),
        }
    }))
    .ok()
}

fn cpp_jump(camera: &GlobeCamera) -> Option<GlobeState> {
    let mut out = GlobeState::default();
    // SAFETY: both pointers are valid for the call.
    (unsafe { ref_globe_jump(camera, &mut out) } == 0).then_some(out)
}

fn check_jump(camera: &GlobeCamera) {
    let (rust, cpp) = (rust_jump(camera), cpp_jump(camera));
    let same_state = match (&rust, &cpp) {
        (Some(r), Some(c)) => {
            r.globe == c.globe
                && same_all(&[r.lat, r.lng, r.zoom, r.bearing, r.pitch], &[c.lat, c.lng, c.zoom, c.bearing, c.pitch])
                && same_all(&r.view_projection, &c.view_projection)
                && same_all(&r.clipping_plane, &c.clipping_plane)
                && same_all(&r.camera_position, &c.camera_position)
        }
        (None, None) => true,
        _ => false,
    };
    assert!(same_state, "jump {camera:?}:\n rust {rust:?}\n c++  {cpp:?}");
}

fn check_screen_to_lat_lng(camera: &GlobeCamera, x: f64, y: f64) {
    let rust = catch_unwind(AssertUnwindSafe(|| {
        let lat_lng =
            set_up(camera).get_state().screen_coordinate_to_lat_lng(&ScreenCoordinate::new(x, y), WrapMode::Unwrapped);
        (lat_lng.latitude(), lat_lng.longitude())
    }))
    .ok();
    let (mut lat, mut lng) = (0.0, 0.0);
    // SAFETY: the pointers are valid for the call.
    let cpp = (unsafe { ref_globe_screen_to_lat_lng(camera, x, y, &mut lat, &mut lng) } == 0).then_some((lat, lng));
    let same_result = match (rust, cpp) {
        (Some(r), Some(c)) => same(r.0, c.0) && same(r.1, c.1),
        (None, None) => true,
        _ => false,
    };
    assert!(same_result, "screenCoordinateToLatLng({x}, {y}) {camera:?}: rust {rust:?} c++ {cpp:?}");
}

fn check_lat_lng_to_screen(camera: &GlobeCamera, lat: f64, lng: f64) {
    let rust = catch_unwind(AssertUnwindSafe(|| {
        let transform = set_up(camera);
        let state = transform.get_state();
        let lat_lng = LatLng::new(lat, lng).expect("LatLng");
        let point = state.lat_lng_to_screen_coordinate(&lat_lng);
        (point.x, point.y, u8::from(state.is_location_occluded(&lat_lng)))
    }))
    .ok();
    let (mut x, mut y, mut occluded) = (0.0, 0.0, 0u8);
    // SAFETY: the pointers are valid for the call.
    let cpp = (unsafe { ref_globe_lat_lng_to_screen(camera, lat, lng, &mut x, &mut y, &mut occluded) } == 0)
        .then_some((x, y, occluded));
    let same_result = match (rust, cpp) {
        (Some(r), Some(c)) => same(r.0, c.0) && same(r.1, c.1) && r.2 == c.2,
        (None, None) => true,
        _ => false,
    };
    assert!(same_result, "latLngToScreenCoordinate({lat}, {lng}) {camera:?}: rust {rust:?} c++ {cpp:?}");
}

fn check_camera(camera: &GlobeCamera) {
    check_jump(camera);

    // A grid over the view and past its edges, where pixels miss the globe.
    let (width, height) = (f64::from(camera.width), f64::from(camera.height));
    for i in 0..=6 {
        for j in 0..=6 {
            let x = width * (-0.25 + 1.5 * f64::from(i) / 6.0);
            let y = height * (-0.25 + 1.5 * f64::from(j) / 6.0);
            check_screen_to_lat_lng(camera, x, y);
        }
    }

    // Near the center, the antipode, the poles and across the antimeridian.
    let lat = camera.lat.clamp(-90.0, 90.0);
    for (dlat, lng) in [
        (0.0, camera.lng),
        (5.0, camera.lng + 10.0),
        (-30.0, camera.lng - 45.0),
        (0.0, camera.lng + 180.0),
        (-lat, camera.lng + 90.0),
    ] {
        check_lat_lng_to_screen(camera, (lat + dlat).clamp(-90.0, 90.0), lng);
    }
    for (lat, lng) in [(90.0, 0.0), (-90.0, 0.0), (0.0, 180.0), (0.0, -180.0), (45.0, 179.999)] {
        check_lat_lng_to_screen(camera, lat, lng);
    }
}

fn camera_strategy() -> impl Strategy<Value = GlobeCamera> {
    (
        prop::sample::select(vec![(256u32, 256u32), (512, 512), (800, 600), (1024, 300), (300, 1024)]),
        prop_oneof![
            -90.0f64..=90.0,
            -85.06f64..=85.06,
            prop::sample::select(vec![90.0, -90.0, 0.0, 85.051128779806604]),
        ],
        prop_oneof![
            -180.0f64..=180.0,
            -540.0f64..=540.0,
            prop::sample::select(vec![180.0, -180.0, 0.0, 179.999, -179.999]),
        ],
        prop_oneof![-4.0f64..20.0, prop::sample::select(vec![-3.0, 0.0, 0.5, 11.0, 12.0])],
        -360.0f64..360.0,
        prop_oneof![0.0f64..85.0, prop::sample::select(vec![0.0, 60.0])],
        prop_oneof![Just(1.0), 0.0f64..=1.0, prop::sample::select(vec![0.5, 0.01, 0.999])],
    )
        .prop_map(|((width, height), lat, lng, zoom, bearing, pitch, transition)| GlobeCamera {
            width,
            height,
            lat,
            lng,
            zoom,
            bearing,
            pitch,
            transition,
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn camera_matches(camera in camera_strategy()) {
        check_camera(&camera);
    }
}

#[test]
fn poles_antimeridian_and_low_zooms() {
    let mut count = 0;
    for &(width, height) in &[(512u32, 512u32), (800, 600)] {
        for &lat in &[90.0, -90.0, 89.0, 85.051128779806604, 0.0, -60.0] {
            for &lng in &[180.0, -180.0, 0.0, 359.0] {
                for &zoom in &[-3.0, -1.0, 0.0, 2.0, 11.5] {
                    for &(bearing, pitch) in &[(0.0, 0.0), (180.0, 60.0), (-45.0, 30.0)] {
                        check_camera(&GlobeCamera { width, height, lat, lng, zoom, bearing, pitch, transition: 1.0 });
                        count += 1;
                    }
                }
            }
        }
    }
    println!("globe cameras compared: {count}");
}
