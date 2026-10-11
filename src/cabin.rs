//! Spawns the cabin, its seats and the initial passengers.

use bevy::prelude::*;

use crate::layout::{self, BUSINESS_ROWS, FEET_ANCHOR, ROWS, SEAT_COLUMNS, SPRITE_SCALE, Facing, WalkTarget};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpawnCabin;

/// Startup configuration of a game round.
#[derive(Resource)]
pub struct CabinConfig {
  /// Fraction of seats (0.0..=1.0) occupied when the game starts.
  pub occupancy: f32,
}

impl Default for CabinConfig {
  fn default() -> Self {
    Self { occupancy: 0.6 }
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SeatClass {
  Business,
  Economy,
}

#[derive(Component)]
pub struct Seat {
  row: usize,
  column: usize,
  pub class: SeatClass,
}

#[derive(Component)]
#[relationship(relationship_target = Occupant)]
pub struct SeatedIn(pub Entity);

#[derive(Component, Deref)]
#[relationship_target(relationship = SeatedIn)]
pub struct Occupant(Entity);

pub struct CabinPlugin;

impl Plugin for CabinPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<CabinConfig>()
      .add_systems(Startup, (spawn_cabin, spawn_seats).chain().in_set(SpawnCabin));
  }
}

fn spawn_cabin(mut commands: Commands, assets: Res<AssetServer>) {
  commands.spawn((
    Name::new("Cabin"),
    Sprite::from_image(assets.load("plane_cabin.png")),
    Transform::from_xyz(0.0, 0.0, layout::BACKGROUND_Z),
    Pickable::IGNORE,
  ));
  // The near wall is drawn on top of seats and passengers so they appear inside the cabin.
  commands.spawn((
    Name::new("Cabin front wall"),
    Sprite::from_image(assets.load("plane_cabin_front_layer.png")),
    Transform::from_xyz(0.0, 0.0, layout::FOREGROUND_Z),
    // Lets clicks through to the seats it overlaps.
    Pickable::IGNORE,
  ));
}

fn spawn_seats(mut commands: Commands, assets: Res<AssetServer>) {
  let blue = assets.load("seat_blue.png");
  let red  = assets.load("seat_red.png");

  for row in 0..ROWS {
    let class = if row >= ROWS - BUSINESS_ROWS {
      SeatClass::Business
    } else {
      SeatClass::Economy
    };
    let image = match class {
      SeatClass::Business => red.clone(),
      SeatClass::Economy => blue.clone(),
    };

    for column in 0..SEAT_COLUMNS.len() {
      let (along, across) = layout::seat_floor_position(row, column);
      commands.spawn((
        Name::new(format!("Seat {}{}", row + 1, (b'A' + column as u8) as char)),
        Seat { row, column, class },
        Sprite::from_image(image.clone()),
        FEET_ANCHOR,
        Transform::from_translation(layout::floor_to_world(along, across))
          .with_scale(Vec3::splat(SPRITE_SCALE)),
        seat_target(row, column),
      ));
    }
  }
}

/// Clicking a seat (or its passenger) sends the attendant to that row in the
/// aisle, where she turns towards the seat.
fn seat_target(row: usize, column: usize) -> WalkTarget {
  let facing = if SEAT_COLUMNS[column] > 0.0 { Facing::Window } else { Facing::Near };
  WalkTarget { spot: Vec2::new(layout::row_along(row), 0.0), facing: Some(facing) }
}
