//! Screen-ray and axis-aligned picking helpers for depth-tested 3D views.

use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray3 {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray3 {
    pub fn at(self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb3 {
    pub fn from_center_size(center: Vec3, size: Vec3) -> Self {
        let half = size * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }
    pub fn intersect(&self, ray: Ray3) -> Option<f32> {
        let mut near: f32 = 0.0;
        let mut far: f32 = f32::INFINITY;
        for axis in 0..3 {
            let origin = ray.origin[axis];
            let direction = ray.direction[axis];
            let min = self.min[axis];
            let max = self.max[axis];
            if direction.abs() < f32::EPSILON {
                if origin < min || origin > max {
                    return None;
                }
                continue;
            }
            let inverse = 1.0 / direction;
            let mut t0 = (min - origin) * inverse;
            let mut t1 = (max - origin) * inverse;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            near = near.max(t0);
            far = far.min(t1);
            if near > far {
                return None;
            }
        }
        Some(near)
    }
}

/// Build a world-space ray from a screen point and a Macroquad camera.
/// `viewport` is `(x, y, width, height)` in physical screen pixels.
pub fn screen_ray(camera: &Camera3D, screen: Vec2, viewport: Option<(i32, i32, i32, i32)>) -> Ray3 {
    let (x, y, width, height) = match viewport {
        Some(viewport) => viewport,
        None => (0, 0, screen_width() as i32, screen_height() as i32),
    };
    let local_x = screen.x - x as f32;
    let local_y = screen.y - y as f32;
    let ndc_x = local_x / width as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - local_y / height as f32 * 2.0;
    let aspect = camera.aspect.unwrap_or(width as f32 / height as f32);
    let view = Mat4::look_at_rh(camera.position, camera.target, camera.up);
    let projection = match camera.projection {
        Projection::Perspective => {
            Mat4::perspective_rh_gl(camera.fovy, aspect, camera.z_near, camera.z_far)
        }
        Projection::Orthographics => {
            let top = camera.fovy / 2.0;
            let right = top * aspect;
            Mat4::orthographic_rh_gl(-right, right, -top, top, camera.z_near, camera.z_far)
        }
    };
    let inverse = (projection * view).inverse();
    let near = inverse.transform_point3(vec3(ndc_x, ndc_y, -1.0));
    let far = inverse.transform_point3(vec3(ndc_x, ndc_y, 1.0));
    Ray3 {
        origin: near,
        direction: (far - near).normalize(),
    }
}

#[cfg(test)]
mod tests;
