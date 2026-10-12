//! On-screen overlay: the mood score and the attendant's task queue.

use bevy::prelude::*;
use strum::IntoEnumIterator;

use crate::attendant::{Task, TaskQueue};
use crate::galley::Appliance;
use crate::passengers::{Mood, Passenger};
use crate::thought_bubble::mood_image;

const MARGIN: f32 = 16.0;
const PADDING: f32 = 12.0;
const CORNER_RADIUS: f32 = 16.0;
const ICON_SIZE: f32 = 32.0;
const FONT_SIZE: f32 = 24.0;

/// Size of `taskqueue_panel.png` in pixels.
const PANEL_SIZE: Vec2 = Vec2::new(500.0, 140.0);
/// Centre of the first slot circle in the panel image, and the distance
/// between two slots.
const FIRST_SLOT_CENTER: Vec2 = Vec2::new(69.5, 69.5);
const SLOT_SPACING: f32 = 120.0;
const SLOTS: usize = 4;
/// Size of the task icons and the active marker. Their circles are drawn so
/// that they line up with the panel's slots at this size.
const SLOT_IMAGE_SIZE: f32 = 100.0;
/// Scale of the task queue panel, relative to its image size.
const PANEL_SCALE: f32 = 0.6;

const BACKGROUND: Color = Color::srgba(1.0, 1.0, 1.0, 0.85);
const OUTLINE: Color = Color::srgb_u8(43, 40, 64);

/// Text showing how many passengers are in this mood.
#[derive(Component)]
struct MoodCount(Mood);

/// Icon showing the task at this position in the queue.
#[derive(Component)]
struct TaskSlot(usize);

/// Ring over the first slot while the attendant has a task.
#[derive(Component)]
struct ActiveMarker;

#[derive(Resource)]
struct TaskIcons {
  walk: Handle<Image>,
  oven: Handle<Image>,
  coffee: Handle<Image>,
  cold_drink: Handle<Image>,
  sandwich: Handle<Image>,
  lavatory: Handle<Image>,
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, (spawn_score, spawn_task_queue))
      .add_systems(Update, (update_counts, update_task_queue));
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

fn spawn_task_queue(mut commands: Commands, assets: Res<AssetServer>) {
  let size = PANEL_SIZE * PANEL_SCALE;
  commands.spawn((
    Name::new("Task queue"),
    ImageNode::new(assets.load("taskqueue_panel.png")),
    Node {
      position_type: PositionType::Absolute,
      left: px(MARGIN),
      bottom: px(MARGIN),
      width: px(size.x),
      height: px(size.y),
      ..default()
    },
  )).with_children(|panel| {
    for index in 0..SLOTS {
      panel.spawn((
        TaskSlot(index),
        ImageNode::default(),
        slot_node(index),
        Visibility::Hidden,
      ));
    }
    // Spawned after the slots, so it is drawn on top of the first icon.
    panel.spawn((
      ActiveMarker,
      ImageNode::new(assets.load("taskqueue_slot_active.png")),
      slot_node(0),
      Visibility::Hidden,
    ));
  });

  commands.insert_resource(TaskIcons {
    walk: assets.load("task_icon_walk.png"),
    oven: assets.load("task_icon_oven.png"),
    coffee: assets.load("task_icon_coffee.png"),
    cold_drink: assets.load("task_icon_cold_drink.png"),
    sandwich: assets.load("task_icon_sandwich.png"),
    lavatory: assets.load("task_icon_lavatory.png"),
  });
}

/// A node covering the slot at `index`, positioned inside the panel.
fn slot_node(index: usize) -> Node {
  let center = FIRST_SLOT_CENTER + Vec2::X * SLOT_SPACING * index as f32;
  let top_left = (center - SLOT_IMAGE_SIZE / 2.0) * PANEL_SCALE;
  Node {
    position_type: PositionType::Absolute,
    left: px(top_left.x),
    top: px(top_left.y),
    width: px(SLOT_IMAGE_SIZE * PANEL_SCALE),
    height: px(SLOT_IMAGE_SIZE * PANEL_SCALE),
    ..default()
  }
}

fn update_task_queue(
  queues: Query<&TaskQueue, Changed<TaskQueue>>,
  appliances: Query<&Appliance>,
  icons: Res<TaskIcons>,
  mut slots: Query<(&TaskSlot, &mut ImageNode, &mut Visibility)>,
  mut marker: Single<&mut Visibility, (With<ActiveMarker>, Without<TaskSlot>)>,
) {
  let Ok(queue) = queues.single() else {
    return;
  };
  let tasks: Vec<Task> = queue.iter().collect();

  for (slot, mut image, mut visibility) in &mut slots {
    match tasks.get(slot.0) {
      Some(&task) => {
        image.image = task_icon(task, &icons, &appliances);
        *visibility = Visibility::Inherited;
      }
      None => *visibility = Visibility::Hidden,
    }
  }
  **marker = if tasks.is_empty() { Visibility::Hidden } else { Visibility::Inherited };
}

fn task_icon(task: Task, icons: &TaskIcons, appliances: &Query<&Appliance>) -> Handle<Image> {
  let icon = match task {
    Task::Use(entity) => match appliances.get(entity) {
      Ok(Appliance::Oven) => &icons.oven,
      Ok(Appliance::CoffeeMaker) => &icons.coffee,
      Ok(Appliance::Fridge) => &icons.cold_drink,
      Ok(Appliance::Counter) => &icons.sandwich,
      Ok(Appliance::Lavatory) => &icons.lavatory,
      // The appliance is gone; the task will fail when it starts.
      Err(_) => &icons.walk,
    },
    Task::Attend(_) | Task::Walk(_) => &icons.walk,
  };
  icon.clone()
}
