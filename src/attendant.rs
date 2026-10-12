//! The flight attendant, who walks to whatever seat row or appliance is clicked.

use std::collections::VecDeque;

use bevy::picking::events::PointerClick;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::cabin::Occupant;
use crate::passengers::Passenger;
use crate::galley::Appliance;
use crate::layout::{self, Facing, SPRITE_SCALE, WalkTarget};

/// Walking speed in floor units per second.
const SPEED: f32 = 120.0;

/// Where the aisle meets the galley. Everything with `along < 0` is galley,
/// where the attendant can walk freely; in the cabin she stays in the aisle.
const AISLE_ENTRY: Vec2 = Vec2::new(0.0, 0.0);

/// Where the attendant waits when the game starts (in the galley, by the aisle).
const START: Vec2 = Vec2::new(-20.0, 0.0);

/// Feet of the attendant sprites, relative to the sprite centre. The figure is
/// not centred horizontally in the image, so this is mirrored when flipping.
const FEET: Vec2 = Vec2::new(0.11, -0.44);

/// How many tasks the attendant can have planned, including the current one.
const MAX_TASKS: usize = 4;

#[derive(Event)] pub struct TaskQueueFull(Task);

#[derive(Component)]
pub struct Attendant {
  pub position: Vec2,
  pub facing: Facing,
  activity: Activity,
}

enum Activity {
  Idle,
  Walking {
    task: Task,
    path: VecDeque<Vec2>,
    facing_on_arrival: Option<Facing>,
  },
}

#[derive(Resource)]
struct AttendantSprites {
  front: Handle<Image>,
  back: Handle<Image>,
}

/// Something the player told the attendant to do. What actually happens is
/// decided on arrival, from the state of the target and what she carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Task {
  /// Operate an appliance.
  Use(Entity),
  /// Go to a passenger and serve whatever she carries, if anything.
  Attend(Entity),
  /// Walk to an empty seat; nothing happens on arrival.
  Walk(Entity),
}

impl Task {
  fn for_target(entity: Entity, is_appliance: bool, is_passenger: bool, occupant: Option<Entity>) -> Self {
    let passenger = if is_passenger { Some(entity) } else { occupant };
    match passenger {
      _ if is_appliance => Task::Use(entity),
      Some(passenger) => Task::Attend(passenger),
      None => Task::Walk(entity),
    }
  }

  pub fn target(self) -> Entity {
    match self {
      Task::Use(entity) | Task::Attend(entity) | Task::Walk(entity) => entity,
    }
  }

  fn is_replaceable(self) -> bool {
    matches!(self, Task::Walk(_) | Task::Attend(_))
  }
}

/// The attendant's planned tasks. The front one is being worked on and stays
/// in the queue until she arrives.
#[derive(Component, Default)]
pub struct TaskQueue {
  tasks: VecDeque<Task>,
}

#[derive(Debug)]
pub struct QueueFull;

impl TaskQueue {
  pub fn push(&mut self, task: Task) -> Result<(), QueueFull> {
    if let Some(last) = self.tasks.back_mut()
      && last.is_replaceable() && task.is_replaceable()
    {
      *last = task;
      return Ok(());
    }
    if self.tasks.len() >= MAX_TASKS {
      return Err(QueueFull);
    }
    self.tasks.push_back(task);
    Ok(())
  }

  pub fn current(&self) -> Option<Task> {
    self.tasks.front().copied()
  }

  pub fn finish_current(&mut self) {
    self.tasks.pop_front();
  }

  pub fn clear(&mut self) {
    self.tasks.clear();
  }

  pub fn iter(&self) -> impl Iterator<Item = Task> + '_ {
    self.tasks.iter().copied()
  }
}

pub struct AttendantPlugin;

impl Plugin for AttendantPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, spawn_attendant)
      .add_systems(Update, (work, update_sprite).chain())
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
      activity: Activity::Idle,
    },
    TaskQueue::default(),
    Sprite::from_image(sprites.front.clone()),
    Anchor(FEET),
    Transform::from_translation(layout::floor_to_world(START.x, START.y))
      .with_scale(Vec3::splat(SPRITE_SCALE)),
    Pickable::IGNORE,
  ));
  commands.insert_resource(sprites);
}

/// Adds the clicked task to the attendant's task queue.
fn direct_attendant(
  click: On<PointerClick>,
  mut commands: Commands,
  targets: Query<(Has<Appliance>, Has<Passenger>, Option<&Occupant>), With<WalkTarget>>,
  mut task_queue: Single<&mut TaskQueue>,
) {
  if click.button != PointerButton::Primary {
    return;
  }
  let Ok((is_appliance, is_passenger, occupant)) = targets.get(click.entity) else {
    return;
  };
  let task = Task::for_target(click.entity, is_appliance, is_passenger, occupant.map(|o| **o));
  if task_queue.push(task).is_err() {
    commands.trigger(TaskQueueFull(task));
  }
}

fn route(from: Vec2, to: Vec2) -> VecDeque<Vec2> {
  let in_galley = |p: Vec2| p.x < AISLE_ENTRY.x;
  let mut path = VecDeque::new();
  if in_galley(from) != in_galley(to) {
    path.push_back(AISLE_ENTRY);
  }
  path.push_back(to);
  path
}

fn work(time: Res<Time>, walk_targets: Query<&WalkTarget>, mut attendant: Single<&mut Attendant>, mut task_queue: Single<&mut TaskQueue>) {
  if matches!(attendant.activity, Activity::Idle) {
    let Some(task) = task_queue.current() else {
      return;
    };
    let Ok(target) = walk_targets.get(task.target()) else {
      // The target is gone (e.g. the passenger left): skip the task.
      task_queue.finish_current();
      return;
    };
    attendant.activity = Activity::Walking {
      task,
      path: route(attendant.position, target.spot),
      facing_on_arrival: target.facing,
    };
  } else {
    let attendant = &mut **attendant;
    let Activity::Walking { task, path, facing_on_arrival } = &mut attendant.activity else {
      return;
    };
    if task_queue.current() != Some(*task) {
      // The current task has changed, stop walking. Next frame the attendant will pick up the new task.
      attendant.activity = Activity::Idle;
    } else {
      let mut budget = SPEED * time.delta_secs();
      while budget > 0.0 {
        let Some(&waypoint) = path.front() else {
          break;
        };
        let delta = waypoint - attendant.position;
        let distance = delta.length();
        if distance > f32::EPSILON {
          attendant.facing = Facing::of_movement(delta);
        }
        if distance <= budget {
          attendant.position = waypoint;
          path.pop_front();
          budget -= distance;
        } else {
          attendant.position += delta / distance * budget;
          budget = 0.0;
        }
      }

      if path.is_empty() {
        if let Some(facing) = *facing_on_arrival {
          attendant.facing = facing;
        }
        attendant.activity = Activity::Idle;
        task_queue.finish_current();
      }
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
