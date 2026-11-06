// extern "C" wrappers around the globe camera of the C++ core
// (src/mln/map/transform.cpp, transform_state.cpp and
// vertical_perspective_projection.cpp) for //difftest/globe. Each call jumps
// a fresh Transform to the camera; globe_difftest.rs does the same in Rust
// and compares the results bit for bit.

#include <mln/map/camera.hpp>
#include <mln/map/transform.hpp>
#include <mln/style/projection_definition.hpp>
#include <mln/util/geo.hpp>

#include <cstdint>
#include <cstring>

using namespace mln;

extern "C" {

struct GlobeCamera {
    uint32_t width;
    uint32_t height;
    double lat;
    double lng;
    double zoom;
    double bearing;
    double pitch;
    // How far the projection is from Mercator to the vertical perspective.
    double transition;
};

struct GlobeState {
    double lat;
    double lng;
    double zoom;
    double bearing;
    double pitch;
    double view_projection[16];
    double clipping_plane[4];
    double camera_position[3];
    uint8_t globe;
};

} // extern "C"

namespace {

void setUp(Transform& transform, const GlobeCamera& camera) {
    transform.resize({camera.width, camera.height});
    transform.setProjectionDefinition(
        ProjectionDefinition(ProjectionType::Mercator, ProjectionType::VerticalPerspective, camera.transition));
    transform.jumpTo(CameraOptions()
                         .withCenter(LatLng{camera.lat, camera.lng})
                         .withZoom(camera.zoom)
                         .withBearing(camera.bearing)
                         .withPitch(camera.pitch));
}

} // namespace

extern "C" {

// Each returns 0, or 1 when the C++ throws.

int ref_globe_jump(const GlobeCamera* camera, GlobeState* out) {
    try {
        Transform transform;
        setUp(transform, *camera);
        const TransformState& state = transform.getState();
        std::memset(out, 0, sizeof(GlobeState));
        const LatLng center = state.getLatLng();
        out->lat = center.latitude();
        out->lng = center.longitude();
        out->zoom = state.getZoom();
        out->bearing = state.getBearing();
        out->pitch = state.getPitch();
        out->globe = state.isGlobeRendering();
        if (state.isGlobeRendering()) {
            std::memcpy(out->view_projection, state.getGlobeViewProjectionMatrix().data(), sizeof(out->view_projection));
            std::memcpy(out->clipping_plane, state.getGlobeClippingPlane().data(), sizeof(out->clipping_plane));
            std::memcpy(out->camera_position, state.getGlobeCameraPosition().data(), sizeof(out->camera_position));
        }
        return 0;
    } catch (...) {
        return 1;
    }
}

int ref_globe_screen_to_lat_lng(const GlobeCamera* camera, double x, double y, double* lat, double* lng) {
    try {
        Transform transform;
        setUp(transform, *camera);
        const LatLng latLng = transform.getState().screenCoordinateToLatLng({x, y}, LatLng::Unwrapped);
        *lat = latLng.latitude();
        *lng = latLng.longitude();
        return 0;
    } catch (...) {
        return 1;
    }
}

int ref_globe_lat_lng_to_screen(
    const GlobeCamera* camera, double lat, double lng, double* x, double* y, uint8_t* occluded) {
    try {
        Transform transform;
        setUp(transform, *camera);
        const TransformState& state = transform.getState();
        const LatLng latLng{lat, lng};
        const ScreenCoordinate point = state.latLngToScreenCoordinate(latLng);
        *x = point.x;
        *y = point.y;
        *occluded = state.isLocationOccluded(latLng);
        return 0;
    } catch (...) {
        return 1;
    }
}

} // extern "C"
