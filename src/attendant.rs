//! The flight attendant, who walks to whatever seat row or appliance is clicked.

use std::collections::VecDeque;

use bevy::picking::events::PointerClick;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::layout::{self, SPRITE_SCALE};

/// Walking speed in floor units per second.
const SPEED: f32 = 90.0;

/// Where the aisle meets the galley. Everything with `along < 0` is galley,
/// where the attendant can walk freely; in the cabin she stays in the aisle.
const AISLE_ENTRY: Vec2 = Vec2::new(0.0, 0.0);

/// Where the attendant waits when the game starts (in the galley, by the aisle).
const START: Vec2 = Vec2::new(-20.0, 0.0);

/// Feet of the attendant sprites, relative to the sprite centre. The figure is
/// not centred horizontally in the image, so this is mirrored when flipping.
const FEET: Vec2 = Vec2::new(0.11, -0.44);

/// Floor spot the attendant walks to when the entity is clicked.
// Sprite picking only considers sprites that have a `Pickable` component.
#[derive(Component, Clone, Copy)]
#[require(Pickable)]
pub struct WalkTarget {
  /// Floor position as `(along, across)`.
  pub spot: Vec2,
  /// Direction to face once arrived; `None` keeps the walking direction.
  pub facing: Option<Facing>,
}

/// Which way the attendant looks, in floor directions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
  /// Towards the front of the plane (+along).
  Front,
  /// Towards the back of the plane (-along).
  Back,
  /// Towards the window wall (+across).
  Window,
  /// Towards the near side, facing the camera (-across).
  Near,
}

impl Facing {
  fn of_movement(delta: Vec2) -> Self {
    if delta.x.abs() >= delta.y.abs() {
      if delta.x >= 0.0 { Facing::Front } else { Facing::Back }
    } else if delta.y >= 0.0 {
      Facing::Window
    } else {
      Facing::Near
    }
  }
}

#[derive(Component)]
pub struct Attendant {
  /// Current floor position as `(along, across)`.
  pub position: Vec2,
  pub facing: Facing,
  /// Remaining waypoints of the current walk.
  path: VecDeque<Vec2>,
  /// Facing to take once the path is finished.
  facing_on_arrival: Option<Facing>,
}

#[derive(Resource)]
struct AttendantSprites {
  front: Handle<Image>,
  back: Handle<Image>,
}

pub struct AttendantPlugin;

impl Plugin for AttendantPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, spawn_attendant)
      .add_systems(Update, (walk, update_sprite).chain())
      .add_observer(direct_attendant);
  }
}

fn spawn_attendant(mut commands: Commands, assets: Res<AssetServer>) {
  let sprites = AttendantSprites {
    front: assets.load("flight_attendant_idle.png"),
    back: assets.load("flight_attendant_idle_back.png"),
  };
  commands.spawn((
    Name::new("Flight attendant"),
    Attendant {
      position: START,
      facing: Facing::Front,
      path: VecDeque::new(),
      facing_on_arrival: None,
    },
    Sprite::from_image(sprites.front.clone()),
    Anchor(FEET),
    Transform::from_translation(layout::floor_to_world(START.x, START.y))
      .with_scale(Vec3::splat(SPRITE_SCALE)),
    // Clicks go through her to the seat or appliance behind.
    Pickable::IGNORE,
  ));
  commands.insert_resource(sprites);
}

/// Sends the attendant to the clicked seat row or appliance.
fn direct_attendant(
  click: On<PointerClick>,
  targets: Query<&WalkTarget>,
  mut attendant: Single<&mut Attendant>,
) {
  if click.button != PointerButton::Primary {
    return;
  }
  let Ok(target) = targets.get(click.entity) else {
    return;
  };
  let path = route(attendant.position, target.spot);
  attendant.path = path;
  attendant.facing_on_arrival = target.facing;
}

/// Waypoints from `from` to `to`. Between the galley and the cabin the walk goes
/// through the aisle entry, so the attendant never cuts through the seat rows.
fn route(from: Vec2, to: Vec2) -> VecDeque<Vec2> {
  let in_galley = |p: Vec2| p.x < AISLE_ENTRY.x;
  let mut path = VecDeque::new();
  if in_galley(from) != in_galley(to) {
    path.push_back(AISLE_ENTRY);
  }
  path.push_back(to);
  path
}

fn walk(time: Res<Time>, mut attendant: Single<&mut Attendant>) {
  let mut budget = SPEED * time.delta_secs();

  while budget > 0.0 {
    let Some(&waypoint) = attendant.path.front() else {
      break;
    };
    let delta = waypoint - attendant.position;
    let distance = delta.length();
    if distance > f32::EPSILON {
      attendant.facing = Facing::of_movement(delta);
    }
    if distance <= budget {
      attendant.position = waypoint;
      attendant.path.pop_front();
      budget -= distance;
    } else {
      attendant.position += delta / distance * budget;
      budget = 0.0;
    }
  }

  if attendant.path.is_empty() {
    if let Some(facing) = attendant.facing_on_arrival.take() {
      attendant.facing = facing;
    }
  }
}

fn update_sprite(
  sprites: Res<AttendantSprites>,
  mut attendants: Query<(&Attendant, &mut Sprite, &mut Anchor, &mut Transform), Changed<Attendant>>,
) {
  for (attendant, mut sprite, mut anchor, mut transform) in &mut attendants {
    // The front sprite looks towards the front of the plane, the back sprite
    // towards the back; mirrored they look across the cabin.
    let (image, flip) = match attendant.facing {
      Facing::Front => (&sprites.front, false),
      Facing::Back => (&sprites.back, false),
      Facing::Window => (&sprites.back, true),
      Facing::Near => (&sprites.front, true),
    };
    if sprite.image != *image {
      sprite.image = image.clone();
    }
    sprite.flip_x = flip;
    anchor.0 = if flip { Vec2::new(-FEET.x, FEET.y) } else { FEET };

    let position = attendant.position;
    transform.translation = layout::floor_to_world(position.x, position.y);
  }
}
