//! Isometric geometry of the cabin.
//!
//! All positions are first expressed in "floor units" on the cabin floor:
//! - `along`: distance along the aisle, growing towards the front of the plane
//!   (screen direction down-right, `(2, 1)` in image pixels)
//! - `across`: distance from the aisle centre line, positive towards the window
//!   wall (screen direction up-right, `(2, -1)` in image pixels)
//!
//! These are then projected onto the background image and converted to world space.

use bevy::prelude::*;
use bevy::sprite::Anchor;

/// Size of `plane_cabin.png` in pixels. The world is laid out so that one world
/// unit equals one background pixel and the background is centred at the origin.
pub const CABIN_SIZE: Vec2 = Vec2::new(1280.0, 812.0);

/// Pixel position (in the background image) of the start of the aisle centre line.
const AISLE_ORIGIN_PX: Vec2 = Vec2::new(450.0, 368.0);
/// Screen offset (in image pixels, y down) of one floor unit along the aisle.
const ALONG_PX: Vec2 = Vec2::new(2.0, 1.0);
/// Screen offset (in image pixels, y down) of one floor unit across the aisle.
const ACROSS_PX: Vec2 = Vec2::new(2.0, -1.0);

/// Number of seat rows in the cabin.
pub const ROWS: usize = 7;
/// Rows at the front of the plane that use the red business-class seats.
pub const BUSINESS_ROWS: usize = 2;
/// Position of the first row along the aisle, level with the first window.
const FIRST_ROW_ALONG: f32 = 36.5;
/// Distance between two rows along the aisle, matching the window spacing
/// (windows are 80px apart horizontally in the background).
const ROW_SPACING: f32 = 40.0;
/// Seat columns as offsets across the aisle; negative is the side nearer the camera.
pub const SEAT_COLUMNS: [f32; 4] = [-64.0, -36.0, 36.0, 64.0];

/// Point of the seat/passenger sprite that touches the floor (relative to the sprite centre).
pub const FEET_ANCHOR: Anchor = Anchor(Vec2::new(0.0, -0.42));

/// Scale applied to seat and passenger sprites.
pub const SPRITE_SCALE: f32 = 0.8;

/// Z layers. Objects on the floor get a z inside `FLOOR_Z` depending on their
/// screen height, so things lower on screen are drawn in front.
pub const BACKGROUND_Z: f32 = 0.0;
pub const FLOOR_Z: std::ops::Range<f32> = 1.0..2.0;
pub const FOREGROUND_Z: f32 = 10.0;
/// Thought bubbles float above everything, keeping the floor depth order among themselves.
pub const BUBBLE_Z: f32 = 20.0;

/// Converts floor coordinates to a world position (including depth).
pub fn floor_to_world(along: f32, across: f32) -> Vec3 {
  let px = AISLE_ORIGIN_PX + along * ALONG_PX + across * ACROSS_PX;
  pixel_to_world(px, px.y)
}

/// Converts a pixel position in the background image to a world position.
/// `depth_y` is the pixel height where the object stands on the floor and
/// decides what it is drawn in front of.
pub fn pixel_to_world(px: Vec2, depth_y: f32) -> Vec3 {
  let depth = (depth_y / CABIN_SIZE.y).clamp(0.0, 1.0);
  let z = FLOOR_Z.start + depth * (FLOOR_Z.end - FLOOR_Z.start);
  Vec3::new(px.x - CABIN_SIZE.x / 2.0, CABIN_SIZE.y / 2.0 - px.y, z)
}

/// Inverse of the floor projection: the floor coordinates `(along, across)`
/// of a pixel in the background image.
pub fn pixel_to_floor(px: Vec2) -> Vec2 {
  // px - origin = along * (2, 1) + across * (2, -1)
  let d = px - AISLE_ORIGIN_PX;
  Vec2::new((d.x / 2.0 + d.y) / 2.0, (d.x / 2.0 - d.y) / 2.0)
}

/// Position of `row` along the aisle.
pub fn row_along(row: usize) -> f32 {
  FIRST_ROW_ALONG + row as f32 * ROW_SPACING
}

/// Floor position of the seat at `row` / `column`.
pub fn seat_floor_position(row: usize, column: usize) -> (f32, f32) {
  (row_along(row), SEAT_COLUMNS[column])
}

/// Floor spot the attendant walks to when the entity is clicked.
// Sprite picking only considers sprites that have a `Pickable` component.
#[derive(Component, Clone, Copy)]
#[require(Pickable)]
pub struct WalkTarget {
  /// Floor position as `(along, across)`.
  pub spot: Vec2,
  /// Direction to face once arrived; `None` keeps the walking direction.
  pub facing: Option<Facing>,
}

/// Which way the attendant looks, in floor directions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
  /// Towards the front of the plane (+along).
  Front,
  /// Towards the back of the plane (-along).
  Back,
  /// Towards the window wall (+across).
  Window,
  /// Towards the near side, facing the camera (-across).
  Near,
}

impl Facing {
  pub fn of_movement(delta: Vec2) -> Self {
    if delta.x.abs() >= delta.y.abs() {
      if delta.x >= 0.0 { Facing::Front } else { Facing::Back }
    } else if delta.y >= 0.0 {
      Facing::Window
    } else {
      Facing::Near
    }
  }
}
