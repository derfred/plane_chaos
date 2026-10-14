use bevy::prelude::*;

use crate::popups::{Popup, PopupProgress, PopupTimer};
use crate::passengers::{Item, Mood, MoodChanged, Desire};

/// Where the bubble's tail points to, in passenger sprite pixels relative to
/// the passenger's feet: just above the top right of the head.
const HEAD_OFFSET: Vec2 = Vec2::new(11.0, 165.0);

const SHOW_DELAY: f32 = 0.5;
const SHOW_TIME: f32 = 3.0;

/// Marks the popup showing a passenger's current desire.
#[derive(Component)]
struct DesireBubble;

fn desire_image(item: Item) -> &'static str {
  match item {
    Item::Coffee => "icon_desire_coffee.png",
    Item::ColdDrink => "icon_desire_cold_drink.png",
    Item::HotMeal => "icon_desire_hot_meal.png",
    Item::Sandwich => "icon_desire_sandwich.png",
  }
}

pub fn mood_image(mood: Mood) -> &'static str {
  match mood {
    Mood::Happy => "icon_mood_happy.png",
    Mood::Neutral => "icon_mood_neutral.png",
    Mood::Unhappy => "icon_mood_unhappy.png",
    Mood::Furious => "icon_mood_furious.png",
  }
}

pub struct ThoughtBubblePlugin;

impl Plugin for ThoughtBubblePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_observer(show_bubble)
      .add_observer(remove_bubble)
      .add_observer(show_mood_bubble)
      .add_systems(Update, update_rings);
  }
}

fn show_bubble(
  insert: On<Insert<Desire>>,
  mut commands: Commands,
  assets: Res<AssetServer>,
  desires: Query<&Desire>,
) {
  let Ok(desire) = desires.get(insert.entity) else {
    return;
  };

  commands.spawn((
    Name::new("Desire bubble"),
    DesireBubble,
    Popup::thought(assets.load(desire_image(desire.item)), Some(desire.fraction_remaining())),
    Transform::from_translation(HEAD_OFFSET.extend(0.0)),
    ChildOf(insert.entity),
  ));
}

fn remove_bubble(
  discard: On<Discard<Desire>>,
  mut commands: Commands,
  passengers: Query<&Children>,
  bubbles: Query<(), With<DesireBubble>>,
) {
  let Ok(children) = passengers.get(discard.entity) else {
    return;
  };
  for child in children.iter() {
    if bubbles.contains(child) {
      commands.entity(child).try_despawn();
    }
  }
}

fn update_rings(
  mut bubbles: Query<(&ChildOf, &mut PopupProgress), With<DesireBubble>>,
  desires: Query<&Desire>,
) {
  for (passenger, mut progress) in &mut bubbles {
    if let Ok(desire) = desires.get(passenger.parent()) {
      progress.set(desire.fraction_remaining());
    }
  }
}

fn show_mood_bubble(
  changed: On<MoodChanged>,
  mut commands: Commands,
  assets: Res<AssetServer>,
) {
  commands.spawn((
    Name::new("Mood bubble"),
    Popup::thought(assets.load(mood_image(changed.new)), None),
    PopupTimer::for_secs(SHOW_TIME).delayed(SHOW_DELAY),
    Transform::from_translation(HEAD_OFFSET.extend(0.0)),
    ChildOf(changed.entity),
  ));
}
