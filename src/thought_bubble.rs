use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::layout;
use crate::passengers::{Item, Mood, Desire};

/// Scale of a bubble on screen.
const BUBBLE_SCALE: f32 = 0.6;

/// Where the bubble's tail points to, in passenger sprite pixels relative to
/// the passenger's feet: just above the top right of the head.
const HEAD_OFFSET: Vec2 = Vec2::new(11.0, 165.0);
const ICON_OFFSET: Vec2 = Vec2::new(9.0, 72.0);

/// Tip of the bubble's tail, relative to the bubble sprite centre.
const TAIL_ANCHOR: Anchor = Anchor(Vec2::new(-0.085, -0.5));

const SHOW_DELAY: f32 = 0.5;
const SHOW_TIME: f32 = 3.0;

const SHADER_PATH: &str = "shaders/timer_ring.wgsl";

/// Size of the ring quad and the ring centre, in bubble sprite pixels relative
/// to the bubble's anchor (the tip of its tail).
const RING_BOX: f32 = 100.0;
const RING_CENTER: Vec2 = Vec2::new(8.0, 74.5);

const GREEN: Color = Color::srgb_u8(76, 195, 106);
const YELLOW: Color = Color::srgb_u8(245, 194, 66);
const RED: Color = Color::srgb_u8(232, 73, 63);
const OUTLINE: Color = Color::srgb_u8(43, 40, 64);

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

#[derive(Resource)]
struct RingMesh(Handle<Mesh>);

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum BubbleKind { Desire, Mood }

#[derive(Component, PartialEq)]
enum MoodPhase { Waiting, Showing }

#[derive(Component)]
struct Delay(Timer);

/// The ring drawn around a bubble, stored on the bubble sprite entity.
#[derive(Component)]
struct TimerRing;

const BUBBLE_BACKGROUND: &str = "bubble_bg.png";

fn desire_image(item: Item) -> &'static str {
  match item {
    Item::Coffee => "icon_coffee.png",
    Item::ColdDrink => "icon_cold_drink.png",
    Item::HotMeal => "icon_hot_meal.png",
    Item::Sandwich => "icon_sandwich.png",
  }
}

fn mood_image(mood: Mood) -> &'static str {
  match mood {
    Mood::Happy => "icon_happy.png",
    Mood::Neutral => "icon_neutral.png",
    Mood::Unhappy => "icon_unhappy.png",
    Mood::Furious => "icon_furious.png",
  }
}

pub struct ThoughtBubblePlugin;

impl Plugin for ThoughtBubblePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(Material2dPlugin::<TimerRingMaterial>::default())
      .add_observer(show_bubble)
      .add_observer(remove_bubble)
      .add_systems(Startup, create_ring_mesh)
      .add_systems(Update, (update_rings, mood_change, update_mood_timer));
  }
}

fn create_ring_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
  commands.insert_resource(RingMesh(meshes.add(Rectangle::new(RING_BOX, RING_BOX))));
}

fn show_bubble(
  insert: On<Insert<Desire>>,
  mut commands: Commands,
  assets: Res<AssetServer>,
  mesh: Res<RingMesh>,
  mut materials: ResMut<Assets<TimerRingMaterial>>,
  passengers: Query<(&Desire, &Transform)>,
) {
  let Ok((desire, transform)) = passengers.get(insert.entity) else {
    return;
  };

  // The bubble is a child, so it inherits the passenger's position and scale.
  let parent_scale = transform.scale;
  let z = (layout::BUBBLE_Z - layout::FLOOR_Z.start) / parent_scale.z;

  commands.spawn((
    Name::new("Desire bubble"),
    Sprite::from_image(assets.load(BUBBLE_BACKGROUND)),
    TAIL_ANCHOR,
    Transform::from_translation(HEAD_OFFSET.extend(z))
      .with_scale(Vec3::splat(BUBBLE_SCALE) / parent_scale),
    BubbleKind::Desire,
    ChildOf(insert.entity),
  )).with_child((
    Name::new("Desire Icon"),
    Sprite::from_image(assets.load(desire_image(desire.item))),
    Transform::from_translation(ICON_OFFSET.extend(0.001)),
  )).with_child((
    Name::new("Timer ring"),
    TimerRing,
    Mesh2d(mesh.0.clone()),
    MeshMaterial2d(materials.add(TimerRingMaterial::new(desire.fraction_remaining()))),
    Transform::from_translation(RING_CENTER.extend(0.001)),
  ));
}

fn remove_bubble(
  discard: On<Discard<Desire>>,
  mut commands: Commands,
  passengers: Query<&Children>,
  kinds: Query<&BubbleKind>,
) {
  let Ok(children) = passengers.get(discard.entity) else {
    return;
  };
  for child in children.iter() {
    if kinds.get(child) == Ok(&BubbleKind::Desire) {
      commands.entity(child).try_despawn();
    }
  }
}

fn update_rings(
  rings: Query<(&ChildOf, &MeshMaterial2d<TimerRingMaterial>), With<TimerRing>>,
  parents: Query<&ChildOf>,
  desires: Query<&Desire>,
  mut materials: ResMut<Assets<TimerRingMaterial>>,
) {
  for (bubble, handle) in &rings {
    let Ok(passenger) = parents.get(bubble.parent()) else {
      continue;
    };
    let Ok(desire) = desires.get(passenger.parent()) else {
      continue;
    };
    if let Some(mut material) = materials.get_mut(&handle.0) {
      material.set_remaining(desire.fraction_remaining());
    }
  }
}

fn mood_change(
  mut commands: Commands,
  assets: Res<AssetServer>,
  query: Query<(Entity, &Transform, &Mood), Changed<Mood>>,
) {

  for (entity, transform, mood) in query {
    if mood != &Mood::Neutral {
      let parent_scale = transform.scale;
      let z = (layout::BUBBLE_Z - layout::FLOOR_Z.start) / parent_scale.z;

      commands.spawn((
        Name::new("Mood bubble"),
        Sprite::from_image(assets.load(BUBBLE_BACKGROUND)),
        TAIL_ANCHOR,
        Transform::from_translation(HEAD_OFFSET.extend(z))
          .with_scale(Vec3::splat(BUBBLE_SCALE) / parent_scale),
        BubbleKind::Mood,
        Visibility::Hidden,
        MoodPhase::Waiting,
        Delay(Timer::from_seconds(SHOW_DELAY, TimerMode::Once)),
        ChildOf(entity),
      )).with_child((
        Name::new("Mood Icon"),
        Sprite::from_image(assets.load(mood_image(*mood))),
        Transform::from_translation(ICON_OFFSET.extend(0.001)),
      ));
    }
  }
}

fn update_mood_timer(
  mut commands: Commands,
  mut query: Query<(Entity, &mut MoodPhase, &mut Delay, &mut Visibility), With<BubbleKind>>,
  time: Res<Time>,
) {
  for (entity, mut phase, mut delay, mut visibility) in &mut query {
    delay.0.tick(time.delta());
    if delay.0.just_finished() {
      if *phase == MoodPhase::Waiting {
        *phase = MoodPhase::Showing;
        *visibility = Visibility::Inherited;
        delay.0.set_duration(Duration::from_secs_f32(SHOW_TIME));
        delay.0.reset();
      } else {
        commands.entity(entity).despawn();
      }
    }
  }
}
