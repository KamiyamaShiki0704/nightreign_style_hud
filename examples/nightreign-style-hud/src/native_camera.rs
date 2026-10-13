//! Rust-owned copies for HUD mathematics. None of these types describes native layout.
use crate::sdk_host;
use shikifsrs_elden_ring::cameras::PerspectiveCameraObservation;
use shikifsrs_runtime::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector4(pub f32, pub f32, pub f32, pub f32);

#[derive(Debug, Clone, Copy)]
pub struct Matrix4(pub Vector4, pub Vector4, pub Vector4, pub Vector4);

#[derive(Debug, Clone, Copy)]
pub struct HudPerspective {
    pub matrix: Matrix4,
    pub fov: f32,
    pub aspect_ratio: f32,
}
impl From<PerspectiveCameraObservation> for HudPerspective {
    fn from(value: PerspectiveCameraObservation) -> Self {
        let rows = value.matrix_bits.map(|row| {
            let [x, y, z, w] = row.map(f32::from_bits);
            Vector4(x, y, z, w)
        });
        Self {
            matrix: Matrix4(rows[0], rows[1], rows[2], rows[3]),
            fov: f32::from_bits(value.frustum_bits[0]),
            aspect_ratio: f32::from_bits(value.frustum_bits[1]),
        }
    }
}
impl HudPerspective {
    pub fn right(&self) -> Vector4 {
        self.matrix.0
    }
    pub fn forward(&self) -> Vector4 {
        self.matrix.2
    }
    pub fn position(&self) -> Vector4 {
        self.matrix.3
    }
}

pub struct HudCamera {
    pub pers_cam_1: Box<HudPerspective>,
}
impl HudCamera {
    pub fn observe() -> Result<Self> {
        sdk_host::with_objects(|objects, _| {
            let cameras = objects
                .cameras()?
                .ok_or(Error::NativeOperationRejected("HUD camera absent"))?;
            let address = cameras.perspectives[0]
                .ok_or(Error::NativeOperationRejected("HUD perspective absent"))?
                .address;
            let camera = objects
                .perspective_camera(address)?
                .ok_or(Error::NativeOperationRejected("HUD perspective absent"))?;
            Ok(Self {
                pers_cam_1: Box::new(camera.into()),
            })
        })
    }
}

pub struct HudWindow {
    pub screen_width: i32,
    pub screen_height: i32,
}
impl HudWindow {
    pub fn observe() -> Result<Self> {
        sdk_host::with_objects(|objects, _| {
            let window = objects
                .window()?
                .ok_or(Error::NativeOperationRejected("HUD window absent"))?;
            Ok(Self {
                screen_width: window.configured_width,
                screen_height: window.configured_height,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copied_rows_preserve_the_owned_hud_projection_convention() {
        let matrix_bits = [
            [1., 2., 3., 4.],
            [5., 6., 7., 8.],
            [9., 10., 11., 12.],
            [13., 14., 15., 16.],
        ]
        .map(|row| row.map(f32::to_bits));
        let copied = HudPerspective::from(PerspectiveCameraObservation {
            camera: shikifsrs_elden_ring::objects::ObjectObservation {
                address: 1,
                kind: shikifsrs_elden_ring::objects::ObjectKind::PerspectiveCamera,
            },
            matrix_bits,
            frustum_bits: [1.25_f32.to_bits(), 1.5_f32.to_bits(), 0, 0],
        });
        assert_eq!(copied.right(), Vector4(1., 2., 3., 4.));
        assert_eq!(copied.forward(), Vector4(9., 10., 11., 12.));
        assert_eq!(copied.position(), Vector4(13., 14., 15., 16.));
        assert_eq!((copied.fov, copied.aspect_ratio), (1.25, 1.5));
    }
}
