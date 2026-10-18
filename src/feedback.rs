//! Short messages that float up over the cabin and fade out.

use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::Text2dShadow;

use crate::layout::{self, SPRITE_SCALE};
use crate::passengers::ServeRejected;
use crate::thought_bubble::HEAD_OFFSET;

const SHOW_TIME: Duration = Duration::from_millis(1200);
/// How far the message drifts up while it fades, in world units.
const RISE: f32 = 30.0;
const FONT_SIZE: f32 = 26.0;

const ERROR_COLOR: Color = Color::srgb_u8(232, 73, 63);
const OUTLINE: Color = Color::srgb_u8(43, 40, 64);

/// Drifts the text up and fades it out, then despawns it.
#[derive(Component)]
struct FadeOut {
  timer: Timer,
  start: Vec3,
}

pub struct FeedbackPlugin;

impl Plugin for FeedbackPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_observer(show_rejected)
      .add_systems(Update, fade_out);
  }
}

fn show_rejected(
  rejected: On<ServeRejected>,
  mut commands: Commands,
  passengers: Query<&Transform>,
) {
  let Ok(passenger) = passengers.get(rejected.entity) else {
    return;
  };
  // Above the passenger's head, where the desire bubble was.
  let start = (passenger.translation.truncate() + HEAD_OFFSET * SPRITE_SCALE).extend(layout::BUBBLE_Z + 1.0);
  commands.spawn((
    Name::new("Wrong order"),
    Text2d::new("Wrong order!"),
    TextFont::default().with_font_size(FONT_SIZE),
    TextColor(ERROR_COLOR),
    Text2dShadow { offset: Vec2::new(2.0, -2.0), color: OUTLINE },
    Transform::from_translation(start),
    FadeOut { timer: Timer::new(SHOW_TIME, TimerMode::Once), start },
  ));
}

fn fade_out(
  mut commands: Commands,
  time: Res<Time>,
  mut messages: Query<(Entity, &mut FadeOut, &mut Transform, &mut TextColor, &mut Text2dShadow)>,
) {
  for (entity, mut fade, mut transform, mut color, mut shadow) in &mut messages {
    if fade.timer.tick(time.delta()).is_finished() {
      commands.entity(entity).despawn();
      continue;
    }
    let fraction = fade.timer.fraction();
    transform.translation = fade.start + Vec3::Y * RISE * fraction;
    // Fully visible for the first half, then fades out.
    let alpha = (2.0 * (1.0 - fraction)).min(1.0);
    color.0.set_alpha(alpha);
    shadow.color.set_alpha(alpha);
  }
}
