//! The galley and lavatory at the back of the plane.

use bevy::prelude::*;

use crate::layout;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Appliance {
  Oven,
  Counter,
  CoffeeMaker,
  Fridge,
  Lavatory,
}

#[derive(Component)]
pub struct Lavatory {
  pub occupied: bool,
}

struct Placement {
  appliance: Appliance,
  image: &'static str,
  /// Centre of the sprite in background pixels.
  center_px: Vec2,
  /// Pixel height used for depth sorting (see [`layout::pixel_to_world`]).
  depth_y: f32,
}

/// Back to front: the lavatory stands in the far corner, the oven nearest the camera.
const GALLEY: [Placement; 5] = [
  Placement {
    appliance: Appliance::Lavatory,
    image: "lavatory_vacant.png",
    center_px: Vec2::new(417.0, 142.0),
    depth_y: 280.0,
  },
  Placement {
    appliance: Appliance::Fridge,
    image: "fridge.png",
    // Front-bottom corner on the counter's right-bottom corner, so both fronts are flush.
    center_px: Vec2::new(299.0, 230.0),
    depth_y: 300.0,
  },
  Placement {
    appliance: Appliance::Counter,
    image: "galley_counter.png",
    center_px: Vec2::new(230.0, 305.0),
    depth_y: 370.0,
  },
  // Stands on the counter, so it only needs to be drawn just in front of it.
  // Its base lies fully inside the counter's top face.
  Placement {
    appliance: Appliance::CoffeeMaker,
    image: "coffee_maker.png",
    center_px: Vec2::new(214.0, 234.0),
    depth_y: 371.0,
  },
  Placement {
    appliance: Appliance::Oven,
    image: "oven.png",
    center_px: Vec2::new(155.0, 327.0),
    depth_y: 380.0,
  },
];

pub struct GalleyPlugin;

impl Plugin for GalleyPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, spawn_galley);
  }
}

fn spawn_galley(mut commands: Commands, assets: Res<AssetServer>) {
  for placement in &GALLEY {
    let mut entity = commands.spawn((
      Name::new(format!("{:?}", placement.appliance)),
      placement.appliance,
      Sprite::from_image(assets.load(placement.image)),
      Transform::from_translation(layout::pixel_to_world(placement.center_px, placement.depth_y)),
    ));
    if placement.appliance == Appliance::Lavatory {
      entity.insert(Lavatory { occupied: false });
    }
  }
}
