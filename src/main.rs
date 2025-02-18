#![feature(const_fn_floating_point_arithmetic)]
#![feature(negative_impls)]
#![feature(vec_into_raw_parts)]
#![feature(box_syntax)]
#![feature(slice_as_chunks)]
#![feature(array_methods)]
#![allow(irrefutable_let_patterns)]

#[allow(unused_imports)]
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::App;
use bevy::DefaultPlugins;
use bevy_embedded_assets::EmbeddedAssetPlugin;
use bevy_framepace::FramepacePlugin;
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};

use crate::ecs::plugins::camera::CameraPlugin;
use crate::ecs::plugins::console::ConsolePlugin;
use crate::ecs::plugins::game::GamePlugin;
use crate::ecs::plugins::imgui::{ImguiPlugin, ImguiState};
use crate::ecs::plugins::preamble::Preamble;
use crate::ecs::plugins::rendering::mesh_pipeline::loader::GltfMeshStorage;
use crate::ecs::plugins::rendering::ShikataganaiRendererPlugins;
use crate::ecs::plugins::settings::SettingsPlugin;

mod ecs;
mod util;

fn main() {
  App::new()
    .add_plugin(SettingsPlugin)
    .add_plugin(Preamble)
    .add_plugins_with(DefaultPlugins, |group| {
      group.add_before::<bevy::asset::AssetPlugin, _>(EmbeddedAssetPlugin)
    })
    .add_plugin(GamePlugin)
    .add_plugin(CameraPlugin)
    .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
    .add_plugin(ImguiPlugin)
    .add_plugins(ShikataganaiRendererPlugins)
    .add_plugin(ConsolePlugin)
    .add_plugin(FramepacePlugin)
    // .add_plugin(RapierDebugRenderPlugin::default())
    // .add_plugin(LogDiagnosticsPlugin::default())
    // .add_plugin(FrameTimeDiagnosticsPlugin::default())
    .run();
}
