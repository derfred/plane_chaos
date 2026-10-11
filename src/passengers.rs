use rand::{RngExt, seq::{IndexedRandom, IteratorRandom}};
use strum::{EnumIter, IntoEnumIterator};
use std::time::Duration;

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

/// Number of passengers that get a random bubble when the game starts.
const INITIAL_BUBBLES: usize = 6;

#[derive(Component)]
#[require(Mood)]
pub struct Passenger;

#[derive(Clone, Copy, PartialEq, Eq, Debug, EnumIter)]
pub enum Item { Coffee, ColdDrink, HotMeal, Sandwich }

impl Item {
  fn random<R: rand::Rng + ?Sized>(rng: &mut R) -> Self {
    Item::iter().choose(rng).unwrap()
  }
}

#[derive(Component)]
pub struct Desire { pub item: Item, patience: Timer }

impl Desire {
  pub fn new(item: Item, duration: Duration) -> Self {
    Self {
      item,
      patience: Timer::from_seconds(duration.as_secs_f32(), TimerMode::Once),
    }
  }

  pub fn fraction_remaining(&self) -> f32 {
    self.patience.fraction_remaining()
  }
}

#[derive(Component, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, EnumIter)]
#[component(immutable)]
pub enum Mood {
  Furious,
  Unhappy,
  #[default] Neutral,
  Happy,
}

impl Mood {
  /// One step down; `Furious` stays `Furious`.
  pub fn worse(self) -> Self {
    match self {
      Mood::Happy => Mood::Neutral,
      Mood::Neutral => Mood::Unhappy,
      Mood::Unhappy | Mood::Furious => Mood::Furious,
    }
  }

  /// One step up; `Happy` stays `Happy`.
  pub fn better(self) -> Self {
    match self {
      Mood::Happy | Mood::Neutral => Mood::Happy,
      Mood::Unhappy => Mood::Neutral,
      Mood::Furious => Mood::Unhappy,
    }
  }
}

/// Outcome events, triggered on the passenger.
#[derive(EntityEvent)] pub struct DesireSatisfied { pub entity: Entity, pub item: Item }
#[derive(EntityEvent)] pub struct DesireExpired   { pub entity: Entity, pub item: Item }

/// Input from the rest of the game: something was handed to this passenger.
#[derive(EntityEvent)] pub struct Serve { pub entity: Entity, pub item: Item }

pub struct PassengersPlugin;

impl Plugin for PassengersPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, spawn_passengers.after(SpawnCabin))
      .add_systems(Update, tick_patience)
      .add_observer(improve_mood)
      .add_observer(worsen_mood);
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

  let mut passengers: Vec<Entity> = Vec::new();
  for index in rand::seq::index::sample(&mut rng, seats.len(), occupied) {
    let (seat_entity, seat_transform, seat_target) = seats[index];

    // Slightly in front of the seat so the passenger is drawn over it.
    let mut transform = *seat_transform;
    transform.translation.z += 0.0001;

    let passenger_entity = commands.spawn((
      Name::new("Passenger"),
      Passenger,
      SeatedIn(seat_entity),
      Sprite::from_image(sprites.choose(&mut rng).unwrap().clone()),
      FEET_ANCHOR,
      transform,
      *seat_target,
    )).id();
    passengers.push(passenger_entity);
  }

  // Assign initial thought bubbles to a subset of passengers.
  for &passenger_entity in passengers.iter().take(INITIAL_BUBBLES) {
    let item = Item::random(&mut rng);
    let duration = Duration::from_secs_f32(rng.random_range(20.0..40.0));
    commands.entity(passenger_entity).insert(Desire::new(item, duration));
  }
}

fn tick_patience(
  mut commands: Commands,
  time: Res<Time>,
  mut query: Query<(Entity, &mut Desire)>,
) {
  for (entity, mut desire) in query.iter_mut() {
    desire.patience.tick(time.delta());
    if desire.patience.just_finished() {
      commands.trigger(DesireExpired { entity, item: desire.item });
      commands.entity(entity).remove::<Desire>();
    }
  }
}

fn improve_mood(satisfied: On<DesireSatisfied>, mut commands: Commands, moods: Query<&Mood>) {
  if let Ok(&mood) = moods.get(satisfied.entity) {
    commands.entity(satisfied.entity).insert(mood.better());
  }
}

fn worsen_mood(expired: On<DesireExpired>, mut commands: Commands, moods: Query<&Mood>) {
  if let Ok(&mood) = moods.get(expired.entity) {
    commands.entity(expired.entity).insert(mood.worse());
  }
}
