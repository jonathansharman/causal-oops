use bevy::prelude::*;
use bevy_pixel_camera::PixelCameraPlugin;

fn main() {
	App::new()
		.add_startup_system(setup)
		.insert_resource(WindowDescriptor {
			title: "Causal Oops".to_string(),
			width: 800.0,
			height: 600.0,
			..Default::default()
		})
		.insert_resource(ClearColor(Color::BLACK))
		// Disable anti-aliasing.
		.insert_resource(Msaa { samples: 1 })
		// Use nearest sampling rather than linear interpolation.
		.insert_resource(bevy::render::texture::ImageSettings::default_nearest())
		.add_plugins(DefaultPlugins)
		.add_plugin(PixelCameraPlugin)
		.run();
}

fn setup() {}
