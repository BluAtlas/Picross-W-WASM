// region:      IMPORTS

#![allow(unused)]

use bevy::input::mouse::MouseButtonInput;
use bevy::input::touch::TouchPhase;
use bevy::input::ButtonState;
use bevy::{prelude::*, render::camera::WindowOrigin};
use board::{BoardAction, BoardPlugin, CurrentAction, InputEvent};
use crossbeam_channel::{unbounded, Receiver, Sender};
use picross_handler::Cell;
use picross_handler::Puzzle;
use std::sync::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

mod board;

// endregion

// region:      GLOBAL

static mut GLOBAL_SENDER: Option<Mutex<Sender<(String, String)>>> = None;
static mut GLOBAL_RECEIVER: Option<Mutex<Receiver<(String, String)>>> = None;

// endregion

// region:      ASSETS

const TILE_SIZE: (f32, f32) = (100., 100.);
const SPRITE_SCALE: f32 = 0.5;

// endregion

// region:      RESOURCES
#[derive(Resource)]
struct GameTextures {
    tile_empty: Handle<Image>,
    tile_filled: Handle<Image>,
    tile_crossed: Handle<Image>,
    tile_clue: Handle<Image>,
    font: Handle<Font>,
}

#[derive(Resource)]
struct WinSize {
    w: f32,
    h: f32,
}

#[derive(Resource)]
struct WASMReceiveChannel {
    rx: Receiver<(String, String)>,
}

#[derive(Resource)]
struct WASMSendChannel {
    tx: Sender<(String, String)>,
}

// endregion

// region:      EVENTS

struct NewBoardEvent {
    clues: String,
    cells: String,
}

struct BoardUpdateEvent(String);

// endregion

fn main() {
    // get size of canvas object in HTML
    let window_elm = web_sys::window().unwrap();
    let device_pixel_ratio = window_elm.device_pixel_ratio() as f32;
    let canvas_elm = window_elm
        .document()
        .unwrap()
        .get_element_by_id("bevy-canvas")
        .unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas_elm
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    let mut canvas_width: f32 = canvas.client_width() as f32;
    let mut canvas_height: f32 = canvas.client_height() as f32;
    if canvas_width * device_pixel_ratio > 4096. {
        canvas_width = (4096. / device_pixel_ratio);
    }
    if canvas_height * device_pixel_ratio > 4096. {
        canvas_height = (4096. / device_pixel_ratio);
    }

    // construct global sender
    let (tx, rx) = unbounded();
    unsafe {
        GLOBAL_SENDER = Some(Mutex::new(tx));
    }

    let receive_channel = WASMReceiveChannel { rx };

    // construct global receiver
    let (tx, rx) = unbounded();
    unsafe {
        GLOBAL_RECEIVER = Some(Mutex::new(rx));
    }

    let send_channel = WASMSendChannel { tx };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: canvas_width,
                height: canvas_height,
                canvas: Some("#bevy-canvas".to_string()),
                fit_canvas_to_parent: true,
                ..Default::default()
            },
            ..Default::default()
        }))
        .add_plugin(BoardPlugin)
        .add_startup_system(setup_system)
        .add_event::<NewBoardEvent>()
        .add_event::<BoardUpdateEvent>()
        .insert_resource(receive_channel)
        .insert_resource(send_channel)
        .add_system(receive_channel_system)
        .run();
}

fn setup_system(mut commands: Commands, asset_server: Res<AssetServer>, windows: Res<Windows>) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            window_origin: WindowOrigin::BottomLeft,
            ..Default::default()
        },
        ..Default::default()
    });

    let window = windows.get_primary().unwrap();

    let win_size = WinSize {
        w: window.width(),
        h: window.height(),
    };

    commands.insert_resource(win_size);

    let game_textures = GameTextures {
        tile_empty: asset_server.load("tiles/tile_empty.png"),
        tile_filled: asset_server.load("tiles/tile_filled.png"),
        tile_crossed: asset_server.load("tiles/tile_crossed.png"),
        tile_clue: asset_server.load("tiles/tile_clue.png"),
        font: asset_server.load("fonts/FOT-NewRodin-Pro-DB.otf"),
    };
    commands.insert_resource(game_textures)
}

fn receive_channel_system(
    receive_channel: Res<WASMReceiveChannel>,
    mut new_board_event_writer: EventWriter<NewBoardEvent>,
    mut board_update_event_writer: EventWriter<BoardUpdateEvent>,
) {
    if let Ok(string) = receive_channel.rx.try_recv() {
        let command: &str = string.0.as_str();
        let data = string.1;
        match command {
            // joined room, new board and cells
            "j" => {
                let mut data_iter = data.split("SPLIT");
                let mut clues = String::from("");
                let mut cells = String::from("");
                if let Some(line) = data_iter.next() {
                    clues = String::from(line);
                }
                if let Some(line) = data_iter.next() {
                    cells = String::from(line);
                }
                new_board_event_writer.send(NewBoardEvent { clues, cells })
            }
            // board update
            "u" => {
                board_update_event_writer.send(BoardUpdateEvent(data));
            }
            // unknown command
            c => {
                warn!("Invalid receive_channel_system, unknown command: {}", c)
            }
        }
    };
}

#[wasm_bindgen]
pub fn send_wasm(command: &str, data: &str) {
    let tx: Sender<(String, String)>;
    unsafe {
        tx = GLOBAL_SENDER.as_ref().unwrap().lock().unwrap().clone();
    }
    tx.send((command.to_string(), data.to_string()));
}

#[wasm_bindgen]
pub fn recv_wasm() -> String {
    let mut result = String::from("");
    let rx: Receiver<(String, String)>;
    unsafe {
        rx = GLOBAL_RECEIVER.as_ref().unwrap().lock().unwrap().clone();
    }

    if let Ok(string) = rx.try_recv() {
        result.push_str(string.0.as_str());
        result.push_str("SPLIT");
        result.push_str(string.1.as_str());
    }

    result
}
