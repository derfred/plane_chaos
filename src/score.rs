use bevy::prelude::*;
use strum::IntoEnumIterator;

use crate::passengers::{Mood, Passenger};
use crate::thought_bubble::mood_image;

const MARGIN: f32 = 16.0;
const PADDING: f32 = 12.0;
const CORNER_RADIUS: f32 = 16.0;
const ICON_SIZE: f32 = 32.0;
const FONT_SIZE: f32 = 24.0;

const BACKGROUND: Color = Color::srgba(1.0, 1.0, 1.0, 0.85);
const OUTLINE: Color = Color::srgb_u8(43, 40, 64);

/// Text showing how many passengers are in this mood.
#[derive(Component)]
struct MoodCount(Mood);

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, spawn_score)
      .add_systems(Update, update_counts);
  }
}

fn spawn_score(mut commands: Commands, assets: Res<AssetServer>) {
  commands.spawn((
    Name::new("Score"),
    Node {
      position_type: PositionType::Absolute,
      top: px(MARGIN),
      right: px(MARGIN),
      padding: UiRect::all(px(PADDING)),
      column_gap: px(PADDING),
      align_items: AlignItems::Center,
      border: UiRect::all(px(2.0)),
      border_radius: BorderRadius::all(px(CORNER_RADIUS)),
      ..default()
    },
    BackgroundColor(BACKGROUND),
    BorderColor::all(OUTLINE),
  )).with_children(|panel| {
    for mood in Mood::iter().rev() {
      panel.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: px(4.0),
        ..default()
      }).with_children(|entry| {
        entry.spawn((
          ImageNode::new(assets.load(mood_image(mood))),
          Node { width: px(ICON_SIZE), height: px(ICON_SIZE), ..default() },
        ));
        entry.spawn((
          MoodCount(mood),
          Text::new("0"),
          TextFont::default().with_font_size(FONT_SIZE),
          TextColor(OUTLINE),
        ));
      });
    }
  });
}

fn update_counts(
  passengers: Query<&Mood, With<Passenger>>,
  mut counts: Query<(&MoodCount, &mut Text)>,
) {
  for (count, mut text) in &mut counts {
    let value = passengers.iter().filter(|&&mood| mood == count.0).count().to_string();
    if text.0 != value {
      text.0 = value;
    }
  }
}
