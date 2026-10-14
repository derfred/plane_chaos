use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite::Anchor;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::layout;

#[derive(Clone, Copy)]
pub enum PopupKind {
  Thought { ring: Option<f32> },
  Spoilt,
  Progress { bar: f32, color: Color, highlight: Color },
}

impl PopupKind {
  /// Initial fraction of the progress indicator, if this kind has one.
  fn progress(&self) -> Option<f32> {
    match *self {
      Self::Thought { ring } => ring,
      Self::Progress { bar, .. } => Some(bar),
      Self::Spoilt => None,
    }
  }
}

#[derive(Component)]
#[require(Sprite, Anchor, Transform, Visibility, Pickable = Pickable::IGNORE)]
#[component(immutable, on_insert = Popup::on_insert, on_discard = Popup::on_discard)]
pub struct Popup {
  pub kind: PopupKind,
  pub icon: Handle<Image>,
}

/// Marks the icon and progress indicator children that a popup's hook spawns.
#[derive(Component)]
struct PopupPart;

impl Popup {
  pub fn thought(icon: Handle<Image>, progress: Option<f32>) -> Self {
    Self {
      kind: PopupKind::Thought { ring: progress },
      icon,
    }
  }

  pub fn spoilt(icon: Handle<Image>) -> Self {
    Self {
      kind: PopupKind::Spoilt,
      icon,
    }
  }

  pub fn progress(icon: Handle<Image>, bar: f32, color: Color, highlight: Color) -> Self {
    Self {
      kind: PopupKind::Progress { bar, color, highlight },
      icon,
    }
  }

  /// Dresses the popup with its background, places it above everything and
  /// spawns its icon and progress indicator.
  fn on_insert(mut world: DeferredWorld, ctx: HookContext) {
    let entity = ctx.entity;
    let Some(popup) = world.get::<Popup>(entity) else {
      return;
    };
    let (kind, icon) = (popup.kind, popup.icon.clone());
    let variant = world.resource::<PopupVariants>().get(&kind).clone();

    // The popup is a child, so cancel the parent's scale to get the same size
    // on any host, and lift it into the bubble layer while keeping the
    // parent's depth order.
    let parent_scale = world
      .get::<ChildOf>(entity)
      .and_then(|child_of| world.get::<Transform>(child_of.parent()))
      .map_or(Vec3::ONE, |transform| transform.scale);

    if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
      sprite.image = variant.image.clone();
    }
    if let Some(mut anchor) = world.get_mut::<Anchor>(entity) {
      *anchor = variant.tail;
    }
    if let Some(mut transform) = world.get_mut::<Transform>(entity) {
      transform.translation.z = (layout::BUBBLE_Z - layout::FLOOR_Z.start) / parent_scale.z;
      transform.scale = Vec3::splat(variant.scale) / parent_scale;
    }

    let indicator = match (kind, &variant.progress) {
      (PopupKind::Thought { ring: Some(fraction) }, Some(ProgressSlot::Ring { center, mesh, .. })) => {
        let material = world.resource_mut::<Assets<RingMaterial>>().add(RingMaterial::new(fraction));
        Some((fraction, *center, mesh.clone(), MaterialHandle::Ring(material)))
      }
      (PopupKind::Progress { bar, color, highlight }, Some(ProgressSlot::Bar { center, mesh, .. })) => {
        let material = world.resource_mut::<Assets<BarMaterial>>().add(BarMaterial {
          color: color.into(),
          highlight_color: highlight.into(),
          outline_color: OUTLINE.into(),
          progress: bar.clamp(0.0, 1.0),
        });
        Some((bar, *center, mesh.clone(), MaterialHandle::Bar(material)))
      }
      _ => {
        debug_assert!(kind.progress().is_none(), "popup background has no slot for its progress");
        None
      }
    };

    let mut commands = world.commands();
    commands.spawn((
      Name::new("Popup icon"),
      PopupPart,
      Sprite::from_image(icon),
      Transform::from_translation(variant.icon.extend(0.001)),
      Pickable::IGNORE,
      ChildOf(entity),
    ));

    if let Some((fraction, center, mesh, material)) = indicator {
      commands.entity(entity).insert(PopupProgress(fraction.clamp(0.0, 1.0)));
      let mut child = commands.spawn((
        Name::new("Popup progress"),
        PopupPart,
        Mesh2d(mesh),
        Transform::from_translation(center.extend(0.002)),
        Pickable::IGNORE,
        ChildOf(entity),
      ));
      match material {
        MaterialHandle::Ring(handle) => child.insert(MeshMaterial2d(handle)),
        MaterialHandle::Bar(handle) => child.insert(MeshMaterial2d(handle)),
      };
    }
  }

  /// Removes what `on_insert` added, before a new `Popup` replaces this one
  /// or the component is removed.
  fn on_discard(mut world: DeferredWorld, ctx: HookContext) {
    let entity = ctx.entity;
    let parts: Vec<Entity> = world
      .get::<Children>(entity)
      .map(|children| children.iter().filter(|&child| world.get::<PopupPart>(child).is_some()).collect())
      .unwrap_or_default();

    let mut commands = world.commands();
    for part in parts {
      commands.entity(part).try_despawn();
    }
    commands.entity(entity).try_remove::<PopupProgress>();
  }
}

enum MaterialHandle {
  Ring(Handle<RingMaterial>),
  Bar(Handle<BarMaterial>),
}

#[derive(Component)]
pub struct PopupProgress(f32); 

impl PopupProgress {
  pub fn get(&self) -> f32 { self.0 }
  pub fn set(&mut self, fraction: f32) { self.0 = fraction.clamp(0.0, 1.0); }
}

/// Shows a popup for a while, then despawns it.
///
/// ```ignore
/// PopupTimer::for_secs(3.0)               // shown right away
/// PopupTimer::for_secs(3.0).delayed(0.5)  // hidden for 0.5s first
/// ```
#[derive(Component)]
pub struct PopupTimer {
  delay: Timer,
  duration: Timer,
}

impl PopupTimer {
  /// Shows the popup right away for `secs` seconds.
  pub fn for_secs(secs: f32) -> Self {
    Self {
      delay: Timer::from_seconds(0.0, TimerMode::Once),
      duration: Timer::from_seconds(secs, TimerMode::Once),
    }
  }

  /// Keeps the popup hidden for `secs` seconds before showing it.
  pub fn delayed(mut self, secs: f32) -> Self {
    self.delay = Timer::from_seconds(secs, TimerMode::Once);
    self
  }
}

const RING_SHADER_PATH: &str = "shaders/timer_ring.wgsl";
const PROGRESS_SHADER_PATH: &str = "shaders/progress_bar.wgsl";

#[derive(Clone)]
struct PopupVariant {
  image: Handle<Image>,
  /// Tip of the tail, relative to the sprite centre. The popup's transform
  /// sits on this point, so it is also the pivot for scaling.
  tail: Anchor,
  /// Scale of the popup on screen.
  scale: f32,
  /// Centre of the icon. Which icon to show is up to the caller.
  icon: Vec2,
  /// Where a progress indicator goes, if this background has room for one.
  progress: Option<ProgressSlot>,
}

#[derive(Clone)]
enum ProgressSlot {
  /// A horizontal bar, drawn with `progress_bar.wgsl`.
  Bar { center: Vec2, mesh: Handle<Mesh> },
  /// A ring around the icon, drawn with `timer_ring.wgsl`.
  Ring { center: Vec2, mesh: Handle<Mesh> },
}

const OUTLINE: Color = Color::srgb_u8(43, 40, 64);
const GREEN: Color = Color::srgb_u8(76, 195, 106);
const YELLOW: Color = Color::srgb_u8(245, 194, 66);
const RED: Color = Color::srgb_u8(232, 73, 63);

/// A ring that empties as time runs out, changing colour on the way.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct RingMaterial {
  #[uniform(0)]
  color: LinearRgba,
  #[uniform(0)]
  outline_color: LinearRgba,
  #[uniform(0)]
  progress: f32,
}

impl RingMaterial {
  fn new(fraction: f32) -> Self {
    let mut material = Self {
      color: LinearRgba::NONE,
      outline_color: OUTLINE.into(),
      progress: 0.0,
    };
    material.set_progress(fraction);
    material
  }

  fn set_progress(&mut self, fraction: f32) {
    let color = if fraction > 0.5 {
      GREEN
    } else if fraction > 0.25 {
      YELLOW
    } else {
      RED
    };
    self.color = color.into();
    self.progress = fraction.clamp(0.0, 1.0);
  }
}

impl Material2d for RingMaterial {
  fn fragment_shader() -> ShaderRef {
    RING_SHADER_PATH.into()
  }

  fn alpha_mode(&self) -> AlphaMode2d {
    AlphaMode2d::Blend
  }
}

/// A horizontal bar that fills up.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct BarMaterial {
  #[uniform(0)]
  color: LinearRgba,
  #[uniform(0)]
  highlight_color: LinearRgba,
  #[uniform(0)]
  outline_color: LinearRgba,
  #[uniform(0)]
  progress: f32,
}

impl Material2d for BarMaterial {
  fn fragment_shader() -> ShaderRef {
    PROGRESS_SHADER_PATH.into()
  }

  fn alpha_mode(&self) -> AlphaMode2d {
    AlphaMode2d::Blend
  }
}

/// All popup backgrounds, loaded once at startup.
#[derive(Resource)]
struct PopupVariants {
  thought: PopupVariant,
  spoilt: PopupVariant,
  progress: PopupVariant,
}

impl PopupVariants {
  fn get(&self, kind: &PopupKind) -> &PopupVariant {
    match kind {
      PopupKind::Thought { .. } => &self.thought,
      PopupKind::Spoilt => &self.spoilt,
      PopupKind::Progress { .. } => &self.progress,
    }
  }
}

impl FromWorld for PopupVariants {
  fn from_world(world: &mut World) -> Self {
    let assets = world.resource::<AssetServer>().clone();
    let mut meshes = world.resource_mut::<Assets<Mesh>>();
    let ring_size = 100.0;
    let thought_ring = ProgressSlot::Ring {
      center: Vec2::new(8.0, 74.5),
      mesh: meshes.add(Rectangle::new(ring_size, ring_size)),
    };
    let bar_size = Vec2::new(76.0, 14.0);
    let bar = ProgressSlot::Bar {
      center: Vec2::new(10.0, 21.5),
      mesh: meshes.add(Rectangle::from_size(bar_size)),
    };

    Self {
      thought: PopupVariant {
        image: assets.load("bubble_bg_thought.png"),
        tail: Anchor(Vec2::new(-0.085, -0.5)),
        scale: 0.6,
        icon: Vec2::new(9.0, 72.0),
        progress: Some(thought_ring),
      },
      spoilt: PopupVariant {
        image: assets.load("bubble_bg_spoilt.png"),
        tail: Anchor(Vec2::new(-0.085, -0.5)),
        scale: 0.6,
        icon: Vec2::new(9.0, 72.0),
        progress: None,
      },
      progress: PopupVariant {
        image: assets.load("bubble_bg_progress.png"),
        tail: Anchor(Vec2::new(0.05, -0.44)),
        scale: 0.8,
        icon: Vec2::new(-46.0, 21.5),
        progress: Some(bar),
      },
    }
  }
}

pub struct PopupsPlugin;

impl Plugin for PopupsPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((Material2dPlugin::<RingMaterial>::default(), Material2dPlugin::<BarMaterial>::default()))
      .init_resource::<PopupVariants>()
      // After `Update`, so progress set by game systems shows in the same frame.
      .add_systems(PostUpdate, (sync_progress, tick_popup_timers));
  }
}

/// Copies each changed `PopupProgress` into its indicator's material.
fn sync_progress(
  popups: Query<(&PopupProgress, &Children), Changed<PopupProgress>>,
  rings: Query<&MeshMaterial2d<RingMaterial>, With<PopupPart>>,
  bars: Query<&MeshMaterial2d<BarMaterial>, With<PopupPart>>,
  mut ring_materials: ResMut<Assets<RingMaterial>>,
  mut bar_materials: ResMut<Assets<BarMaterial>>,
) {
  for (progress, children) in &popups {
    for child in children.iter() {
      if let Ok(handle) = rings.get(child) {
        if let Some(mut material) = ring_materials.get_mut(&handle.0) {
          material.set_progress(progress.get());
        }
      } else if let Ok(handle) = bars.get(child) {
        if let Some(mut material) = bar_materials.get_mut(&handle.0) {
          material.progress = progress.get();
        }
      }
    }
  }
}

/// Hides timed popups until their delay is over and despawns them once they
/// have been shown long enough. Runs in `PostUpdate`, so a popup spawned during
/// `Update` is hidden before its first frame is drawn.
fn tick_popup_timers(
  mut commands: Commands,
  time: Res<Time>,
  mut popups: Query<(Entity, &mut PopupTimer, &mut Visibility)>,
) {
  for (entity, mut timer, mut visibility) in &mut popups {
    if timer.delay.is_finished() {
      timer.duration.tick(time.delta());
    } else {
      timer.delay.tick(time.delta());
    }
    visibility.set_if_neq(if timer.delay.is_finished() { Visibility::Inherited } else { Visibility::Hidden });
    if timer.duration.is_finished() {
      commands.entity(entity).try_despawn();
    }
  }
}
