use rand::{RngExt, seq::{IndexedRandom, IteratorRandom}};
use strum::{EnumIter, IntoEnumIterator};
use std::time::Duration;

use bevy::prelude::*;

use crate::layout::{FEET_ANCHOR, WalkTarget};
use crate::cabin::{CabinConfig, Seat, SeatClass, SeatedIn, SpawnCabin};

const PASSENGER_SPRITES: [&str; 6] = [
  "passenger_bearded.png",
  "passenger_businessman.png",
  "passenger_grandpa.png",
  "passenger_kid.png",
  "passenger_woman_blonde.png",
  "passenger_woman_bun.png",
];

const BUSINESS_IMPATIENCE_FACTOR: f32 = 0.75;
const MAX_DESIRE_RESET: f32 = 30.0;
const MIN_DESIRE_RESET: f32 = 10.0;

/// Most desires active across the cabin at the same time.
const MAX_ACTIVE_DESIRES: usize = 6;
/// Seconds between two desires handed out by the [`DesireDirector`].
const MIN_DESIRE_INTERVAL: f32 = 4.0;
const MAX_DESIRE_INTERVAL: f32 = 8.0;

#[derive(Component)]
#[require(Mood)]
pub struct Passenger {
  /// Runs down while the passenger has no desire; until it is finished the
  /// director passes them over.
  desire_cooldown: Timer,
}

fn desire_cooldown<R: rand::Rng + ?Sized>(rng: &mut R) -> Timer {
  Timer::from_seconds(rng.random_range(MIN_DESIRE_RESET..MAX_DESIRE_RESET), TimerMode::Once)
}

/// Paces desires across the whole cabin: once per interval it hands a desire
/// to a random rested passenger, as long as fewer than `max_active` are active.
#[derive(Resource)]
pub struct DesireDirector {
  next: Timer,
  max_active: usize,
}

impl Default for DesireDirector {
  fn default() -> Self {
    Self { next: desire_interval(&mut rand::rng()), max_active: MAX_ACTIVE_DESIRES }
  }
}

fn desire_interval<R: rand::Rng + ?Sized>(rng: &mut R) -> Timer {
  Timer::from_seconds(rng.random_range(MIN_DESIRE_INTERVAL..MAX_DESIRE_INTERVAL), TimerMode::Once)
}

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

fn item_patience(item: Item) -> f32 {
  match item {
    Item::Coffee => 30.0,
    Item::ColdDrink => 20.0,
    Item::HotMeal => 40.0,
    Item::Sandwich => 20.0,
  }
}

pub struct PassengersPlugin;

impl Plugin for PassengersPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<DesireDirector>()
      .add_systems(Startup, spawn_passengers.after(SpawnCabin))
      // Expired desires are removed first, so their slot is free for the director.
      .add_systems(Update, (tick_patience, cool_down_passengers, direct_desires).chain())
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
      Passenger {
        desire_cooldown: desire_cooldown(&mut rng),
      },
      SeatedIn(seat_entity),
      Sprite::from_image(sprites.choose(&mut rng).unwrap().clone()),
      FEET_ANCHOR,
      transform,
      *seat_target,
    )).id();
    passengers.push(passenger_entity);
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

fn cool_down_passengers(time: Res<Time>, mut passengers: Query<&mut Passenger, Without<Desire>>) {
  for mut passenger in &mut passengers {
    passenger.desire_cooldown.tick(time.delta());
  }
}

/// Once the interval is up, gives a random rested passenger a desire. While the
/// cabin is at its limit or nobody is rested, the director stays due and
/// hands out the desire as soon as it can.
fn direct_desires(
  mut commands: Commands,
  time: Res<Time>,
  mut director: ResMut<DesireDirector>,
  active: Query<(), With<Desire>>,
  mut passengers: Query<(Entity, &mut Passenger, &SeatedIn), Without<Desire>>,
  seats: Query<&Seat>,
) {
  director.next.tick(time.delta());
  if !director.next.is_finished() || active.iter().count() >= director.max_active {
    return;
  }

  let mut rng = rand::rng();
  let Some((entity, mut passenger, seated_in)) = passengers
    .iter_mut()
    .filter(|(_, passenger, _)| passenger.desire_cooldown.is_finished())
    .choose(&mut rng)
  else {
    return;
  };

  let item = Item::random(&mut rng);
  let mut patience = item_patience(item);
  if seats.get(seated_in.0).is_ok_and(|seat| seat.class == SeatClass::Business) {
    patience *= BUSINESS_IMPATIENCE_FACTOR;
  }
  commands.entity(entity).insert(Desire::new(item, Duration::from_secs_f32(patience)));

  // The cooldown only runs once this desire is over.
  passenger.desire_cooldown = desire_cooldown(&mut rng);
  director.next = desire_interval(&mut rng);
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
