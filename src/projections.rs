use bevy::{
	camera::{CameraProjection, OrthographicProjection, SubCameraView},
	math::{Mat4, Vec2, Vec3A},
};

/// An oblique [`CameraProjection`].
#[derive(Debug, Clone)]
pub struct ObliqueProjection {
	/// Distance from the camera where skew is zero.
	pub focal_distance: f32,
	pub obliqueness: Vec2,
	pub orthographic: OrthographicProjection,
}

impl ObliqueProjection {
	fn skew(&self, mut mat: Mat4) -> Mat4 {
		let skew = self.obliqueness / self.orthographic.area.size();

		mat.col_mut(2)[0] = skew.x;
		mat.col_mut(2)[1] = skew.y;

		mat.col_mut(3)[0] += skew.x * self.focal_distance;
		mat.col_mut(3)[1] += skew.y * self.focal_distance;

		mat
	}
}

impl CameraProjection for ObliqueProjection {
	fn get_clip_from_view(&self) -> Mat4 {
		self.skew(self.orthographic.get_clip_from_view())
	}

	fn get_clip_from_view_for_sub(&self, sub_view: &SubCameraView) -> Mat4 {
		self.skew(self.orthographic.get_clip_from_view_for_sub(sub_view))
	}

	fn update(&mut self, width: f32, height: f32) {
		self.orthographic.update(width, height);
	}

	fn far(&self) -> f32 {
		self.orthographic.far
	}

	fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
		self.orthographic.get_frustum_corners(z_near, z_far)
	}
}
