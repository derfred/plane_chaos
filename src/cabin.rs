//! Spawns the cabin, its seats and the initial passengers.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use rand::seq::IndexedRandom;

use crate::layout::{self, BUSINESS_ROWS, ROWS, SEAT_COLUMNS, SPRITE_SCALE, Facing, WalkTarget};

const PASSENGER_SPRITES: [&str; 6] = [
  "passenger_bearded.png",
  "passenger_businessman.png",
  "passenger_grandpa.png",
  "passenger_kid.png",
  "passenger_woman_blonde.png",
  "passenger_woman_bun.png",
];

/// Point of the seat/passenger sprite that touches the floor (relative to the sprite centre).
const FEET_ANCHOR: Anchor = Anchor(Vec2::new(0.0, -0.42));

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
  pub row: usize,
  pub column: usize,
  pub class: SeatClass,
  /// The passenger currently sitting in this seat.
  pub occupant: Option<Entity>,
}

#[derive(Component)]
pub struct Passenger {
  pub seat: Option<Entity>,
}

pub struct CabinPlugin;

impl Plugin for CabinPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<CabinConfig>()
      .add_systems(Startup, (spawn_cabin, spawn_seats, spawn_passengers).chain());
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
        Seat { row, column, class, occupant: None },
        Sprite::from_image(image.clone()),
        FEET_ANCHOR,
        Transform::from_translation(layout::floor_to_world(along, across))
          .with_scale(Vec3::splat(SPRITE_SCALE)),
        seat_target(row, column),
      ));
    }
  }
}

fn spawn_passengers(
  mut commands: Commands,
  assets: Res<AssetServer>,
  config: Res<CabinConfig>,
  mut seats: Query<(Entity, &mut Seat, &Transform, &WalkTarget)>,
) {
  let mut rng = rand::rng();
  let sprites: Vec<Handle<Image>> = PASSENGER_SPRITES.iter().map(|path| assets.load(*path)).collect();

  let mut seats: Vec<_> = seats.iter_mut().collect();
  let occupied = (seats.len() as f32 * config.occupancy.clamp(0.0, 1.0)).round() as usize;

  for index in rand::seq::index::sample(&mut rng, seats.len(), occupied) {
    let (seat_entity, seat, seat_transform, seat_target) = &mut seats[index];

    // Slightly in front of the seat so the passenger is drawn over it.
    let mut transform = **seat_transform;
    transform.translation.z += 0.0001;

    let passenger = commands
      .spawn((
        Name::new("Passenger"),
        Passenger { seat: Some(*seat_entity) },
        Sprite::from_image(sprites.choose(&mut rng).unwrap().clone()),
        FEET_ANCHOR,
        transform,
        **seat_target,
      ))
      .id();
    seat.occupant = Some(passenger);
  }
}

/// Clicking a seat (or its passenger) sends the attendant to that row in the
/// aisle, where she turns towards the seat.
fn seat_target(row: usize, column: usize) -> WalkTarget {
  let facing = if SEAT_COLUMNS[column] > 0.0 { Facing::Window } else { Facing::Near };
  WalkTarget { spot: Vec2::new(layout::row_along(row), 0.0), facing: Some(facing) }
}
