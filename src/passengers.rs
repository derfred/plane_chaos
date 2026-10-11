use rand::seq::IndexedRandom;
use bevy::prelude::*;

use crate::layout::{FEET_ANCHOR, WalkTarget};
use crate::cabin::{CabinConfig, Seat, SeatedIn, SpawnCabin};

const PASSENGER_SPRITES: [&str; 6] = [
  "passenger_bearded.png",
  "passenger_businessman.png",
  "passenger_grandpa.png",
  "passenger_kid.png",
  "passenger_woman_blonde.png",
  "passenger_woman_bun.png",
];

#[derive(Component)]
pub struct Passenger;

enum Item { Coffee, ColdDrink, HotMeal, Sandwich }
struct Desire { item: Item, patience: Timer }
enum Mood { Content, Happy, Unhappy }

pub struct PassengersPlugin;

impl Plugin for PassengersPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, spawn_passengers.after(SpawnCabin));
  }
}

fn spawn_passengers(
  mut commands: Commands,
  assets: Res<AssetServer>,
  config: Res<CabinConfig>,
  seats: Query<(Entity, &Transform, &WalkTarget), With<Seat>>,
) {
  let mut rng = rand::rng();
  let sprites: Vec<Handle<Image>> = PASSENGER_SPRITES.iter().map(|path| assets.load(*path)).collect();

  let seats: Vec<_> = seats.iter().collect();
  let occupied = (seats.len() as f32 * config.occupancy.clamp(0.0, 1.0)).round() as usize;

  for index in rand::seq::index::sample(&mut rng, seats.len(), occupied) {
    let (seat_entity, seat_transform, seat_target) = seats[index];

    // Slightly in front of the seat so the passenger is drawn over it.
    let mut transform = *seat_transform;
    transform.translation.z += 0.0001;

    commands.spawn((
      Name::new("Passenger"),
      Passenger,
      SeatedIn(seat_entity),
      Sprite::from_image(sprites.choose(&mut rng).unwrap().clone()),
      FEET_ANCHOR,
      transform,
      *seat_target,
    ));
  }
}
