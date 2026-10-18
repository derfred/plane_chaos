use std::collections::VecDeque;

use bevy::picking::events::PointerClick;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::cabin::Occupant;
use crate::passengers::{Passenger, Item, Serve};
use crate::galley::{Appliance, Filled, StartAppliance, interact_with_appliance};
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

#[derive(Event)] pub struct TaskQueueFull;

#[derive(Component)]
pub struct Attendant {
  pub position: Vec2,
  pub facing: Facing,
  activity: Activity,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WalkProgress {
  OnTheWay,
  /// The last waypoint was reached; she is idle and faces the target.
  Arrived,
}

impl Attendant {
  #[must_use]
  fn walk_on_path(&mut self, delta_secs: f32) -> WalkProgress {
    let Activity::Walking { path, facing_on_arrival, .. } = &mut self.activity else {
      return WalkProgress::OnTheWay;
    };

    let mut budget = SPEED * delta_secs;
    while budget > 0.0 {
      let Some(&waypoint) = path.front() else {
        break;
      };
      let delta = waypoint - self.position;
      let distance = delta.length();
      if distance > f32::EPSILON {
        self.facing = Facing::of_movement(delta);
      }
      if distance <= budget {
        self.position = waypoint;
        path.pop_front();
        budget -= distance;
      } else {
        self.position += delta / distance * budget;
        budget = 0.0;
      }
    }
    if path.is_empty() {
      self.facing = *facing_on_arrival;
      self.activity = Activity::Idle;
      return WalkProgress::Arrived;
    }
    WalkProgress::OnTheWay
  }
}

enum Activity {
  Idle,
  Walking {
    task: Task,
    path: VecDeque<Vec2>,
    facing_on_arrival: Facing,
  },
}

#[derive(Resource)]
struct AttendantSprites {
  front_idle: Handle<Image>,
  back_idle: Handle<Image>,
  front_tray: Handle<Image>,
  back_tray: Handle<Image>,
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

  pub fn finish_current(&mut self) -> Task {
    self.tasks.pop_front().expect("No current task to finish")
  }

  pub fn clear(&mut self) {
    self.tasks.clear();
  }

  pub fn iter(&self) -> impl Iterator<Item = Task> + '_ {
    self.tasks.iter().copied()
  }
}

#[derive(Component)]
struct Tray(Item);

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
    front_idle: assets.load("flight_attendant_idle_front.png"),
    back_idle: assets.load("flight_attendant_idle_back.png"),
    front_tray: assets.load("flight_attendant_tray_front.png"),
    back_tray: assets.load("flight_attendant_tray_back.png"),
  };
  commands.spawn((
    Name::new("Flight attendant"),
    Attendant {
      position: START,
      facing: Facing::Front,
      activity: Activity::Idle,
    },
    TaskQueue::default(),
    Sprite::from_image(sprites.front_idle.clone()),
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
    commands.trigger(TaskQueueFull);
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

fn plan_next_activity(position: Vec2, task_queue: &mut TaskQueue, walk_targets: &Query<&WalkTarget>) -> Activity {
  let Some(task) = task_queue.current() else {
    return Activity::Idle;
  };
  let Ok(target) = walk_targets.get(task.target()) else {
    // The target is gone (e.g. the passenger left): skip the task.
    task_queue.finish_current();
    return Activity::Idle;
  };
  Activity::Walking {
    task,
    path: route(position, target.spot),
    facing_on_arrival: target.facing,
  }
}

fn work(
  time: Res<Time>,
  mut commands: Commands,
  walk_targets: Query<&WalkTarget>,
  attendant: Single<(Entity, &mut Attendant, Option<&Tray>)>,
  mut task_queue: Single<&mut TaskQueue>,
  mut appliances: Query<(&Appliance, Option<&mut Filled>)>,
) {
  // Nothing to do: return before the mutable deref, so she isn't marked as changed.
  let (attendant_entity, mut attendant, tray) = attendant.into_inner();
  if matches!(attendant.activity, Activity::Idle) && task_queue.current().is_none() {
    return;
  }
  let attendant = &mut *attendant;

  match attendant.activity {
    Activity::Idle => {
      attendant.activity = plan_next_activity(attendant.position, &mut task_queue, &walk_targets);
    }
    Activity::Walking { task, .. } => {
      if task_queue.current() != Some(task) {
        // current task has changed, stop walking. Next frame the attendant will pick up the new task.
        attendant.activity = Activity::Idle;
        return;
      }
      if attendant.walk_on_path(time.delta_secs()) == WalkProgress::Arrived {
        match (task_queue.finish_current(), tray) {
          // Her hands are full: she can still switch on an empty appliance.
          (Task::Use(target), Some(_)) => commands.trigger(StartAppliance(target)),
          (Task::Use(target), None) => {
            if let Ok((appliance, filled)) = appliances.get_mut(target)
              && let Some(tray_item) = interact_with_appliance(&mut commands, target, appliance, filled) {
                commands.entity(attendant_entity).insert(Tray(tray_item));
              }
          }
          // Whether the passenger takes it or not, the tray is empty afterwards.
          (Task::Attend(passenger), Some(&Tray(item))) => {
            commands.trigger(Serve { entity: passenger, item });
            commands.entity(attendant_entity).remove::<Tray>();
          }
          (Task::Attend(_) | Task::Walk(_), _) => {}
        }
      }
    }
  }
}

type SpriteOutdated = Or<(Changed<Attendant>, Changed<Tray>)>;

fn update_sprite(
  sprites: Res<AttendantSprites>,
  mut attendants: Query<(&Attendant, &mut Sprite, &mut Anchor, &mut Transform, Has<Tray>), SpriteOutdated>,
) {
  for (attendant, mut sprite, mut anchor, mut transform, has_tray) in &mut attendants {
    let (front, back) = if has_tray {
      (&sprites.front_tray, &sprites.back_tray)
    } else {
      (&sprites.front_idle, &sprites.back_idle)
    };
    // The front sprite looks towards the front of the plane, the back sprite
    // towards the back; mirrored they look across the cabin.
    let (image, flip) = match attendant.facing {
      Facing::Front => (front, false),
      Facing::Back => (back, false),
      Facing::Window => (back, true),
      Facing::Near => (front, true),
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
