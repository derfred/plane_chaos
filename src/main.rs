mod attendant;
mod popups;
mod thought_bubble;
mod passengers;
mod cabin;
mod galley;
mod layout;
mod hud;

use bevy::camera::ScalingMode;
use bevy::prelude::*;

use popups::PopupsPlugin;
use attendant::AttendantPlugin;
use thought_bubble::ThoughtBubblePlugin;
use cabin::{CabinConfig, CabinPlugin};
use galley::GalleyPlugin;
use passengers::PassengersPlugin;
use hud::HudPlugin;

fn main() {
  App::new()
    .add_plugins(
      DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
          title: "Plane Chaos".into(),
          resolution: (1280, 812).into(),
          ..default()
        }),
        ..default()
      }),
    )
    .insert_resource(ClearColor(Color::srgb(0.53, 0.75, 0.92)))
    .insert_resource(CabinConfig { occupancy: 0.6 })
    .add_plugins((CabinPlugin, GalleyPlugin, AttendantPlugin, PassengersPlugin, ThoughtBubblePlugin, PopupsPlugin, HudPlugin))
    .add_systems(Startup, spawn_camera)
    .run();
}

fn spawn_camera(mut commands: Commands) {
  commands.spawn((
    Camera2d,
    // Always fit the whole cabin into the window, whatever its size.
    Projection::Orthographic(OrthographicProjection {
      scaling_mode: ScalingMode::AutoMin {
        min_width: layout::CABIN_SIZE.x,
        min_height: layout::CABIN_SIZE.y,
      },
      ..OrthographicProjection::default_2d()
    }),
  ));
}
