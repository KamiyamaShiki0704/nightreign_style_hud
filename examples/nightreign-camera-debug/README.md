# Nightreign Camera Debug

Standalone debug DLL for camera, projection, and sight-ray research.

## Usage

- Build package: `cargo build --package nightreign-camera-debug --release --target x86_64-pc-windows-msvc`
- Output DLL: `target/x86_64-pc-windows-msvc/release/nightreign_camera_debug.dll`
- Load the DLL only while debugging. The overlay is always active when loaded.

## Overlay Data

- `cam`: camera position, camera mask, forward/right/up vectors, FOV, aspect, near/far, yaw, and pitch.
- `player`: player physics position, hit height, ray origin, and camera-relative depth/side/up values.
- `ray`: center camera ray hit state, filter, distance, world hit point, and projected screen point.
- `param`: current GameMan camera parameter IDs.
- `zoom`: CameraZoomOut state from TAE event 152 fields.
- `force`: ForceCameraDirection state from TAE event 153 fields.

## Visual Markers

- White cross: screen center.
- Orange line: screen-center view ray to the current ray hit or 60m far sample.
- Cyan line/dots: projected sight ray from the player upper-body ray origin.
- Yellow marker: center ray wall/ground hit.

This DLL does not modify gameplay state.
