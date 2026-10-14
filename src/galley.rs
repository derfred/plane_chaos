//! The galley and lavatory at the back of the plane.

use bevy::prelude::*;
use strum::{EnumIter, IntoEnumIterator};
use std::time::Duration;

use crate::layout::{self, Facing, WalkTarget};
use crate::popups::{Popup, PopupProgress, PopupTimer};

const COFFEE_MAKER_DURATION: f32 = 1.0;
const OVEN_DURATION: f32 = 2.0;
const COFFEE_MAKER_SPOIL: f32 = 10.0;
const OVEN_SPOIL: f32 = 5.0;
const COFFEE_MAKER_ITEMS: usize = 10;
const OVEN_ITEMS: usize = 4;

/// How long the spoilt popup stays up.
const SPOILT_SHOW_TIME: f32 = 3.0;

struct PopupStyle {
  /// Icon of the progress popup.
  icon: &'static str,
  /// Icon of the popup shown when the appliance's contents spoil.
  spoilt_icon: Option<&'static str>,
  /// Progress bar colours, matching the appliance.
  color: Color,
  highlight: Color,
  /// Where the tail points to, relative to the appliance sprite centre.
  offset: Vec2,
}

fn popup_style(appliance: Appliance) -> Option<PopupStyle> {
  match appliance {
    Appliance::CoffeeMaker => Some(PopupStyle {
      icon: "icon_progress_coffee.png",
      spoilt_icon: Some("icon_spoilt_coffee.png"),
      color: Color::srgb_u8(138, 85, 48),
      highlight: Color::srgb_u8(192, 138, 94),
      // Just above the machine.
      offset: Vec2::new(0.0, 54.0),
    }),
    Appliance::Oven => Some(PopupStyle {
      icon: "icon_progress_oven.png",
      spoilt_icon: Some("icon_spoilt_hot_meal.png"),
      color: Color::srgb_u8(255, 138, 31),
      highlight: Color::srgb_u8(255, 196, 107),
      // Left of centre, so the popup doesn't hide the coffee maker behind it.
      offset: Vec2::new(-15.0, 47.0),
    }),
    Appliance::Lavatory => Some(PopupStyle {
      icon: "icon_progress_lavatory.png",
      spoilt_icon: None,
      color: Color::srgb_u8(47, 184, 207),
      highlight: Color::srgb_u8(143, 227, 240),
      // On its front face; the cabin ends right above the lavatory.
      offset: Vec2::new(-27.0, 32.0),
    }),
    Appliance::Counter | Appliance::Fridge => None,
  }
}

/// Marks the popup showing an appliance's progress.
#[derive(Component)]
struct ProgressPopup;

#[derive(Component, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, EnumIter)]
pub enum ApplianceState {
  #[default]
  Empty,
  InProgress,
  Filled
}

#[derive(EntityEvent)] pub struct ApplianceStarted(pub Entity);
#[derive(EntityEvent)] pub struct ApplianceFinished(pub Entity);
#[derive(EntityEvent)] pub struct ApplianceSpoiled(pub Entity);

#[derive(Component, Clone, PartialEq, Debug)]
pub struct ApplianceProgress {
  pub fraction: f32, /// How far along with its current job, `0.0..=1.0`.
  pub state: ApplianceState,
  pub items: usize,
  timer: Timer,
}

impl ApplianceProgress {
  pub fn empty() -> Self {
    let mut timer = Timer::from_seconds(0.0, TimerMode::Once);
    timer.pause();
    Self {
      fraction: 0.0,
      state: ApplianceState::Empty,
      items: 0,
      timer
    }
  }

  pub fn tick(&mut self, delta: Duration) {
    self.timer.tick(delta);
    self.fraction = self.timer.fraction();
  }

  pub fn complete_operation(&mut self, appliance: Appliance) {
    let duration = match (appliance, self.state) {
      (Appliance::CoffeeMaker, ApplianceState::Empty) => COFFEE_MAKER_DURATION,
      (Appliance::Oven, ApplianceState::Empty) => OVEN_DURATION,
      (Appliance::CoffeeMaker, ApplianceState::InProgress) => COFFEE_MAKER_SPOIL,
      (Appliance::Oven, ApplianceState::InProgress) => OVEN_SPOIL,
      _ => 0.0,
    };

    self.state = match self.state {
      ApplianceState::Empty => ApplianceState::InProgress,
      ApplianceState::InProgress => ApplianceState::Filled,
      ApplianceState::Filled => ApplianceState::Empty,
    };

    self.timer.reset();
    self.timer.set_duration(Duration::from_secs_f32(duration));

    if self.state == ApplianceState::Empty {
      self.timer.pause();
      self.items = 0;
    } else {
      self.timer.unpause();
      self.items = match appliance {
        Appliance::CoffeeMaker => COFFEE_MAKER_ITEMS,
        Appliance::Oven => OVEN_ITEMS,
        _ => 0,
      };
    }
  }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Appliance {
  Oven,
  Counter,
  CoffeeMaker,
  Fridge,
  Lavatory,
}

struct Placement {
  appliance: Appliance,
  /// Centre of the sprite in background pixels.
  center_px: Vec2,
  /// Pixel height of the centre of its floor footprint, used for depth sorting
  /// (see [`layout::pixel_to_world`]).
  depth_y: f32,
  /// Where the attendant stands to use it, in background pixels (her feet).
  stand_px: Vec2,
}

/// Back to front: the lavatory stands in the far corner, the oven nearest the camera.
const GALLEY: [Placement; 5] = [
  Placement {
    appliance: Appliance::Lavatory,
    center_px: Vec2::new(417.0, 142.0),
    depth_y: 240.0,
    stand_px: Vec2::new(500.0, 281.0),
  },
  Placement {
    appliance: Appliance::Fridge,
    // Front-bottom corner on the counter's right-bottom corner, so both fronts are flush.
    center_px: Vec2::new(299.0, 230.0),
    depth_y: 304.0,
    stand_px: Vec2::new(351.0, 330.0),
  },
  Placement {
    appliance: Appliance::Counter,
    center_px: Vec2::new(230.0, 305.0),
    depth_y: 346.0,
    stand_px: Vec2::new(302.0, 358.0),
  },
  // Stands on the counter, so it only needs to be drawn just in front of it.
  // Its base lies fully inside the counter's top face.
  Placement {
    appliance: Appliance::CoffeeMaker,
    center_px: Vec2::new(214.0, 234.0),
    depth_y: 347.0,
    stand_px: Vec2::new(262.0, 378.0),
  },
  Placement {
    appliance: Appliance::Oven,
    center_px: Vec2::new(155.0, 327.0),
    depth_y: 357.0,
    stand_px: Vec2::new(215.0, 388.0),
  },
];

fn appliance_image(appliance: Appliance, state: ApplianceState) -> &'static str {
  match appliance {
    Appliance::Fridge => "fridge.png",
    Appliance::Counter => "galley_counter.png",
    Appliance::Lavatory => match state {
      ApplianceState::Empty => "lavatory_vacant.png",
      ApplianceState::InProgress => "lavatory_occupied.png",
      ApplianceState::Filled => "lavatory_occupied.png",
    },
    Appliance::CoffeeMaker => match state {
      ApplianceState::Empty => "coffee_maker_empty.png",
      ApplianceState::InProgress => "coffee_maker_inprogress.png",
      ApplianceState::Filled => "coffee_maker_filled.png",
    },
    Appliance::Oven => match state {
      ApplianceState::Empty => "oven_empty.png",
      ApplianceState::InProgress => "oven_inprogress.png",
      ApplianceState::Filled => "oven_filled.png",
    },
  }
}

pub struct GalleyPlugin;

impl Plugin for GalleyPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, spawn_galley)
      .add_observer(update_appliance_state::<ApplianceStarted>)
      .add_observer(update_appliance_state::<ApplianceFinished>)
      .add_observer(update_appliance_state::<ApplianceSpoiled>)
      .add_systems(Update, appliance_progress)
      .add_observer(show_popup)
      .add_observer(remove_popup)
      .add_observer(show_spoilt_popup)
      .add_systems(Update, update_progress_bars);
  }
}

#[derive(Resource)]
struct ApplianceImages(#[allow(dead_code)] Vec<Handle<Image>>);

fn spawn_galley(mut commands: Commands, assets: Res<AssetServer>) {
  let images = GALLEY
    .iter()
    .flat_map(|placement| ApplianceState::iter().map(|state| appliance_image(placement.appliance, state)))
    .map(|path| assets.load(path))
    .collect();
  commands.insert_resource(ApplianceImages(images));

  for placement in &GALLEY {
    let mut entity = commands.spawn((
      Name::new(format!("{:?}", placement.appliance)),
      placement.appliance,
      Sprite::from_image(assets.load(appliance_image(placement.appliance, ApplianceState::Empty))),
      Transform::from_translation(layout::pixel_to_world(placement.center_px, placement.depth_y)),
      WalkTarget { spot: layout::pixel_to_floor(placement.stand_px), facing: Facing::Back },
    ));
    if let Appliance::CoffeeMaker | Appliance::Oven = placement.appliance {
      entity.insert(ApplianceProgress::empty());
    }
  }
}

fn appliance_progress(mut commands: Commands, time: Res<Time>, mut query: Query<(Entity, &Appliance, &mut ApplianceProgress)>) {
  for (entity, appliance, mut progress) in &mut query {
    progress.tick(time.delta());
    if progress.timer.just_finished() {
      progress.complete_operation(*appliance);
      match progress.state {
        ApplianceState::InProgress => commands.trigger(ApplianceStarted(entity)),
        ApplianceState::Filled => commands.trigger(ApplianceFinished(entity)),
        ApplianceState::Empty => commands.trigger(ApplianceSpoiled(entity)),
      }
    }
  }
}


fn update_appliance_state<E: EntityEvent>(evt: On<E>, assets: Res<AssetServer>, mut query: Query<(&Appliance, &ApplianceProgress, &mut Sprite)>) {
  if let Ok((appliance, progress, mut sprite)) = query.get_mut(evt.event_target()) {
    sprite.image = assets.load(appliance_image(*appliance, progress.state));
  }
}

fn show_popup(
  started: On<ApplianceStarted>,
  mut commands: Commands,
  assets: Res<AssetServer>,
  appliances: Query<&Appliance>,
) {
  let entity = started.0;
  let Ok(appliance) = appliances.get(entity) else {
    return;
  };
  let Some(style) = popup_style(*appliance) else {
    warn!("{appliance:?} has no progress popup");
    return;
  };

  commands.spawn((
    Name::new("Progress popup"),
    ProgressPopup,
    Popup::progress(assets.load(style.icon), 0.0, style.color, style.highlight),
    Transform::from_translation(style.offset.extend(0.0)),
    ChildOf(entity),
  ));
}

fn remove_popup(
  finished: On<ApplianceFinished>,
  mut commands: Commands,
  appliances: Query<&Children>,
  popups: Query<(), With<ProgressPopup>>,
) {
  let Ok(children) = appliances.get(finished.0) else {
    return;
  };
  for child in children.iter() {
    if popups.contains(child) {
      commands.entity(child).try_despawn();
    }
  }
}

fn update_progress_bars(
  mut popups: Query<(&ChildOf, &mut PopupProgress), With<ProgressPopup>>,
  appliances: Query<&ApplianceProgress>,
) {
  for (appliance, mut progress) in &mut popups {
    let Ok(appliance) = appliances.get(appliance.parent()) else {
      continue;
    };
    if progress.get() != appliance.fraction {
      progress.set(appliance.fraction);
    }
  }
}

fn show_spoilt_popup(
  spoiled: On<ApplianceSpoiled>,
  mut commands: Commands,
  assets: Res<AssetServer>,
  appliances: Query<&Appliance>,
) {
  let entity = spoiled.0;
  let Ok(appliance) = appliances.get(entity) else {
    return;
  };
  let Some(PopupStyle { spoilt_icon: Some(icon), offset, .. }) = popup_style(*appliance) else {
    warn!("{appliance:?} has no spoilt popup");
    return;
  };

  commands.spawn((
    Name::new("Spoilt popup"),
    Popup::spoilt(assets.load(icon)),
    PopupTimer::for_secs(SPOILT_SHOW_TIME),
    Transform::from_translation(offset.extend(0.0)),
    ChildOf(entity),
  ));
}
