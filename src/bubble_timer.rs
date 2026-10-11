//! Countdown rings around thought bubbles.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::bubble::BubbleSprite;

const SHADER_PATH: &str = "shaders/timer_ring.wgsl";

/// Size of the ring quad and the ring centre, in bubble sprite pixels relative
/// to the bubble's anchor (the tip of its tail).
const RING_BOX: f32 = 100.0;
const RING_CENTER: Vec2 = Vec2::new(8.0, 74.5);

const GREEN: Color = Color::srgb_u8(76, 195, 106);
const YELLOW: Color = Color::srgb_u8(245, 194, 66);
const RED: Color = Color::srgb_u8(232, 73, 63);
const OUTLINE: Color = Color::srgb_u8(43, 40, 64);

/// Add to a passenger with a [`ThoughtBubble`](crate::bubble::ThoughtBubble) to
/// draw a countdown ring around the bubble. The ring empties as the timer runs
/// down and disappears when it is finished.
#[derive(Component)]
pub struct BubbleTimer(pub Timer);

impl BubbleTimer {
  pub fn new(duration: Duration) -> Self {
    Self(Timer::new(duration, TimerMode::Once))
  }

  /// Fraction of the time still left, 1.0 when started, 0.0 when finished.
  pub fn remaining(&self) -> f32 {
    self.0.fraction_remaining()
  }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct TimerRingMaterial {
  #[uniform(0)]
  color: LinearRgba,
  #[uniform(0)]
  outline_color: LinearRgba,
  #[uniform(0)]
  progress: f32,
}

impl TimerRingMaterial {
  fn new(remaining: f32) -> Self {
    let mut material = Self {
      color: LinearRgba::NONE,
      outline_color: OUTLINE.into(),
      progress: 0.0,
    };
    material.set_remaining(remaining);
    material
  }

  fn set_remaining(&mut self, remaining: f32) {
    let color = if remaining > 0.5 {
      GREEN
    } else if remaining > 0.25 {
      YELLOW
    } else {
      RED
    };
    self.color = color.into();
    self.progress = remaining;
  }
}

impl Material2d for TimerRingMaterial {
  fn fragment_shader() -> ShaderRef {
    SHADER_PATH.into()
  }

  fn alpha_mode(&self) -> AlphaMode2d {
    AlphaMode2d::Blend
  }
}

/// The ring drawn around a bubble, stored on the bubble sprite entity.
#[derive(Component)]
struct TimerRing(Entity);

#[derive(Resource)]
struct RingMesh(Handle<Mesh>);

pub struct BubbleTimerPlugin;

impl Plugin for BubbleTimerPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(Material2dPlugin::<TimerRingMaterial>::default())
      .add_systems(Startup, create_ring_mesh)
      .add_systems(Update, (tick_timers, attach_rings, update_rings).chain())
      .add_observer(remove_ring);
  }
}

fn create_ring_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
  commands.insert_resource(RingMesh(meshes.add(Rectangle::new(RING_BOX, RING_BOX))));
}

fn tick_timers(time: Res<Time>, mut timers: Query<&mut BubbleTimer>) {
  for mut timer in &mut timers {
    timer.0.tick(time.delta());
  }
}

/// Spawns a ring for every timed bubble that does not have one yet. This also
/// covers a bubble being replaced while its timer keeps running.
fn attach_rings(
  mut commands: Commands,
  mesh: Res<RingMesh>,
  mut materials: ResMut<Assets<TimerRingMaterial>>,
  timed: Query<(&BubbleTimer, &BubbleSprite)>,
  without_ring: Query<(), Without<TimerRing>>,
) {
  for (timer, sprite) in &timed {
    if !without_ring.contains(sprite.0) {
      continue;
    }
    let ring = commands
      .spawn((
        Name::new("Timer ring"),
        Mesh2d(mesh.0.clone()),
        MeshMaterial2d(materials.add(TimerRingMaterial::new(timer.remaining()))),
        // Just in front of the bubble.
        Transform::from_translation(RING_CENTER.extend(0.001)),
        ChildOf(sprite.0),
      ))
      .id();
    commands.entity(sprite.0).insert(TimerRing(ring));
  }
}

fn update_rings(
  timed: Query<(&BubbleTimer, &BubbleSprite)>,
  rings: Query<&TimerRing>,
  ring_materials: Query<&MeshMaterial2d<TimerRingMaterial>>,
  mut materials: ResMut<Assets<TimerRingMaterial>>,
) {
  for (timer, sprite) in &timed {
    let Ok(ring) = rings.get(sprite.0) else {
      continue;
    };
    let Ok(handle) = ring_materials.get(ring.0) else {
      continue;
    };
    if let Some(mut material) = materials.get_mut(&handle.0) {
      material.set_remaining(timer.remaining());
    }
  }
}

/// Removes the ring when the timer is removed. When the bubble itself goes
/// away, the ring is despawned with it as its child.
fn remove_ring(
  discard: On<Discard<BubbleTimer>>,
  mut commands: Commands,
  sprites: Query<&BubbleSprite>,
  rings: Query<&TimerRing>,
) {
  let Ok(sprite) = sprites.get(discard.entity) else {
    return;
  };
  if let Ok(ring) = rings.get(sprite.0) {
    commands.entity(ring.0).try_despawn();
    commands.entity(sprite.0).try_remove::<TimerRing>();
  }
}
