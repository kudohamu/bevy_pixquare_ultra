use bevy::{
  app::{Plugin, PostUpdate},
  asset::{Assets, Handle},
  ecs::{
    change_detection::DetectChanges,
    component::{Component, Mutable},
    system::{Query, Res},
    world::Ref,
  },
  log::debug,
  sprite::Sprite,
};

use crate::loader::PxArtwork;

#[derive(Component, Default)]
pub struct PixquareFile {
  pub artwork: Handle<PxArtwork>,
}

trait RenderPx {
  type Extra<'e>;

  fn render_px(&mut self, artwork: &PxArtwork, _extra: &mut Self::Extra<'_>);
}

impl RenderPx for Sprite {
  type Extra<'e> = ();

  fn render_px(&mut self, artwork: &PxArtwork, _extra: &mut Self::Extra<'_>) {
    debug!("render!");
  }
}

fn render<T: RenderPx + Component<Mutability = Mutable>>(
  mut q_px: Query<(&mut T, Ref<PixquareFile>)>,
  res_pxartworks: Res<Assets<PxArtwork>>,
  mut extra: <T as RenderPx>::Extra<'_>,
) {
  if !res_pxartworks.is_changed() {
    return;
  }

  for (mut target, px) in q_px.iter_mut() {
    if !px.is_changed() {
      return;
    }
    let Some(artwork) = res_pxartworks.get(&px.artwork) else {
      continue;
    };

    target.render_px(artwork, &mut extra);
  }
}

#[derive(Debug)]
pub struct PixquareRendererPlugin;

impl Plugin for PixquareRendererPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.add_systems(PostUpdate, render::<Sprite>);
  }
}
