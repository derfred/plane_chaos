//! The galley and lavatory at the back of the plane.

use bevy::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

use crate::layout::{self, Facing, WalkTarget};
use crate::popups::{Popup, PopupProgress, PopupTimer};

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

/// What an appliance looks like; derived from its state components.
#[derive(Clone, Copy, PartialEq, Eq, Debug, EnumIter)]
enum ApplianceState {
  Empty,
  InProgress,
  Filled,
}

/// What an appliance produces and how long it takes. Fixed per appliance.
#[derive(Component, Clone, Copy, Debug)]
pub struct Recipe {
  prepare_secs: f32,
  spoil_secs: f32,
  items: usize,
}

fn recipe(appliance: Appliance) -> Option<Recipe> {
  match appliance {
    Appliance::CoffeeMaker => Some(Recipe { prepare_secs: 1.0, spoil_secs: 10.0, items: 10 }),
    Appliance::Oven => Some(Recipe { prepare_secs: 2.0, spoil_secs: 5.0, items: 4 }),
    Appliance::Counter | Appliance::Fridge | Appliance::Lavatory => None,
  }
}

/// The appliance is preparing its items. An appliance without `Preparing` or
/// [`Filled`] is empty.
#[derive(Component, Debug)]
pub struct Preparing(Timer);

/// The appliance holds ready items, which spoil when the timer runs out.
#[derive(Component, Debug)]
pub struct Filled {
  pub items: usize,
  spoils: Timer,
}

/// Input from the rest of the game: start preparing on this appliance. Ignored
/// unless the appliance has a [`Recipe`] and is empty.
#[derive(EntityEvent)] pub struct StartAppliance(pub Entity);

/// The items in this appliance spoiled; it is empty again.
#[derive(EntityEvent)] pub struct ApplianceSpoiled(pub Entity);

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
      .add_observer(start_appliance)
      .add_systems(Update, (tick_preparing, tick_spoiling, update_progress_bars).chain())
      .add_observer(show_preparing)
      .add_observer(show_filled)
      .add_observer(show_empty)
      .add_observer(show_popup)
      .add_observer(remove_popup)
      .add_observer(show_spoilt_popup);
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
    if let Some(recipe) = recipe(placement.appliance) {
      entity.insert(recipe);
    }
  }
}

fn start_appliance(
  start: On<StartAppliance>,
  mut commands: Commands,
  appliances: Query<&Recipe, (Without<Preparing>, Without<Filled>)>,
) {
  if let Ok(recipe) = appliances.get(start.0) {
    let timer = Timer::from_seconds(recipe.prepare_secs, TimerMode::Once);
    commands.entity(start.0).insert(Preparing(timer));
  }
}

fn tick_preparing(
  mut commands: Commands,
  time: Res<Time>,
  mut appliances: Query<(Entity, &mut Preparing, &Recipe)>,
) {
  for (entity, mut preparing, recipe) in &mut appliances {
    if preparing.0.tick(time.delta()).just_finished() {
      commands.entity(entity).remove::<Preparing>().insert(Filled {
        items: recipe.items,
        spoils: Timer::from_seconds(recipe.spoil_secs, TimerMode::Once),
      });
    }
  }
}

fn tick_spoiling(
  mut commands: Commands,
  time: Res<Time>,
  mut appliances: Query<(Entity, &mut Filled)>,
) {
  for (entity, mut filled) in &mut appliances {
    if filled.spoils.tick(time.delta()).just_finished() {
      commands.entity(entity).remove::<Filled>();
      commands.trigger(ApplianceSpoiled(entity));
    }
  }
}

fn show_preparing(insert: On<Insert<Preparing>>, assets: Res<AssetServer>, sprites: Query<(&Appliance, &mut Sprite)>) {
  set_image(insert.entity, ApplianceState::InProgress, &assets, sprites);
}

fn show_filled(insert: On<Insert<Filled>>, assets: Res<AssetServer>, sprites: Query<(&Appliance, &mut Sprite)>) {
  set_image(insert.entity, ApplianceState::Filled, &assets, sprites);
}

fn show_empty(discard: On<Discard<Filled>>, assets: Res<AssetServer>, sprites: Query<(&Appliance, &mut Sprite)>) {
  set_image(discard.entity, ApplianceState::Empty, &assets, sprites);
}

fn set_image(entity: Entity, state: ApplianceState, assets: &AssetServer, mut sprites: Query<(&Appliance, &mut Sprite)>) {
  if let Ok((appliance, mut sprite)) = sprites.get_mut(entity) {
    sprite.image = assets.load(appliance_image(*appliance, state));
  }
}

fn show_popup(
  insert: On<Insert<Preparing>>,
  mut commands: Commands,
  assets: Res<AssetServer>,
  appliances: Query<&Appliance>,
) {
  let entity = insert.entity;
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
  discard: On<Discard<Preparing>>,
  mut commands: Commands,
  appliances: Query<&Children>,
  popups: Query<(), With<ProgressPopup>>,
) {
  let Ok(children) = appliances.get(discard.entity) else {
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
  appliances: Query<&Preparing>,
) {
  for (appliance, mut progress) in &mut popups {
    let Ok(preparing) = appliances.get(appliance.parent()) else {
      continue;
    };
    let fraction = preparing.0.fraction();
    if progress.get() != fraction {
      progress.set(fraction);
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
