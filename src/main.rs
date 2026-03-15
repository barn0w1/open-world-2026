//! Entry point for the FPS walkthrough game.
//!
//! Responsibilities:
//! - Configure `DefaultPlugins` with WASM-friendly window settings
//! - Register the three game plugins (Player, Camera, World)

mod camera;
mod player;
mod terrain;
mod world;

use bevy::{
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

use camera::CameraPlugin;
use player::PlayerPlugin;
use terrain::chunk_manager::TerrainPlugin;
use world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "FPS Walkthrough".into(),

                    // Disables vsync-style blocking waits. On WASM this avoids
                    // stalls caused by the browser's rAF scheduling mismatch.
                    present_mode: PresentMode::AutoNoVsync,

                    // Makes the canvas fill its CSS parent (the <body> in index.html).
                    fit_canvas_to_parent: true,

                    // Must match <canvas id="bevy-canvas"> in index.html.
                    // Bevy's winit backend uses this CSS selector to find the canvas.
                    canvas: Some("#bevy-canvas".to_string()),

                    // Keep browser keyboard shortcuts (F5, Ctrl+R) working while
                    // the game is running. Flip to `true` for a production build
                    // where you want full keyboard capture.
                    prevent_default_event_handling: false,

                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins((PlayerPlugin, CameraPlugin, WorldPlugin, TerrainPlugin))
        .run();
}
