//! Thought bubbles above seated passengers.

use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::Anchor;
use rand::RngExt;
use rand::seq::{IndexedRandom, IteratorRandom};

use crate::bubble_timer::BubbleTimer;
use crate::cabin::Passenger;
use crate::layout;

/// Number of passengers that get a random bubble when the game starts.
const INITIAL_BUBBLES: usize = 6;

/// Scale of a bubble on screen.
const BUBBLE_SCALE: f32 = 0.6;

/// Where the bubble's tail points to, in passenger sprite pixels relative to
/// the passenger's feet: just above the top right of the head.
const HEAD_OFFSET: Vec2 = Vec2::new(11.0, 185.0);

/// Tip of the bubble's tail, relative to the bubble sprite centre.
const TAIL_ANCHOR: Anchor = Anchor(Vec2::new(-0.085, -0.5));

/// Add to a passenger to show a thought bubble above their head. Replacing it
/// changes the bubble, removing it removes the bubble.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThoughtBubble {
  Coffee,
  ColdDrink,
  HotMeal,
  Sandwich,
  Happy,
  Unhappy,
}

impl ThoughtBubble {
  pub const ALL: [ThoughtBubble; 6] = [
    ThoughtBubble::Coffee,
    ThoughtBubble::ColdDrink,
    ThoughtBubble::HotMeal,
    ThoughtBubble::Sandwich,
    ThoughtBubble::Happy,
    ThoughtBubble::Unhappy,
  ];

  fn image(self) -> &'static str {
    match self {
      ThoughtBubble::Coffee => "bubble_coffee.png",
      ThoughtBubble::ColdDrink => "bubble_cold_drink.png",
      ThoughtBubble::HotMeal => "bubble_hot_meal.png",
      ThoughtBubble::Sandwich => "bubble_sandwich.png",
      ThoughtBubble::Happy => "bubble_happy.png",
      ThoughtBubble::Unhappy => "bubble_unhappy.png",
    }
  }
}

/// The child entity rendering a passenger's [`ThoughtBubble`].
#[derive(Component)]
pub(crate) struct BubbleSprite(pub(crate) Entity);

pub struct BubblePlugin;

impl Plugin for BubblePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_observer(show_bubble)
      .add_observer(hide_bubble)
      // Passengers are spawned during Startup.
      .add_systems(PostStartup, assign_initial_bubbles);
  }
}

fn show_bubble(
  insert: On<Insert<ThoughtBubble>>,
  mut commands: Commands,
  assets: Res<AssetServer>,
  passengers: Query<(&ThoughtBubble, &Transform)>,
) {
  let Ok((bubble, transform)) = passengers.get(insert.entity) else {
    return;
  };
  // The bubble is a child, so it inherits the passenger's position and scale.
  let parent_scale = transform.scale;
  let z = (layout::BUBBLE_Z - layout::FLOOR_Z.start) / parent_scale.z;
  let sprite = commands
    .spawn((
      Name::new("Thought bubble"),
      Sprite::from_image(assets.load(bubble.image())),
      TAIL_ANCHOR,
      Transform::from_translation(HEAD_OFFSET.extend(z))
        .with_scale(Vec3::splat(BUBBLE_SCALE) / parent_scale),
      ChildOf(insert.entity),
    ))
    .id();
  commands.entity(insert.entity).insert(BubbleSprite(sprite));
}

/// Runs when the bubble is replaced or removed, and when the passenger is despawned
/// (hence the `try_` variants: the entities may already be gone).
fn hide_bubble(discard: On<Discard<ThoughtBubble>>, mut commands: Commands, sprites: Query<&BubbleSprite>) {
  if let Ok(sprite) = sprites.get(discard.entity) {
    commands.entity(sprite.0).try_despawn();
    commands.entity(discard.entity).try_remove::<BubbleSprite>();
  }
}

/// Gives a few random passengers a random bubble, to see them rendered. Every
/// other bubble gets a timer that is already partly run down, so all ring
/// colours show up.
fn assign_initial_bubbles(mut commands: Commands, passengers: Query<Entity, With<Passenger>>) {
  let mut rng = rand::rng();
  for (index, passenger) in passengers.iter().sample(&mut rng, INITIAL_BUBBLES).into_iter().enumerate() {
    let bubble = *ThoughtBubble::ALL.choose(&mut rng).unwrap();
    commands.entity(passenger).insert(bubble);
    if index % 2 == 0 {
      let duration = Duration::from_secs_f32(rng.random_range(20.0..40.0));
      let mut timer = BubbleTimer::new(duration);
      timer.0.set_elapsed(duration.mul_f32(rng.random_range(0.0..0.8)));
      commands.entity(passenger).insert(timer);
    }
  }
}
