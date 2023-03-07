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

static mut GLOBAL_SENDER: Option<Mutex<Sender<(String, String)>>> = None;

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
struct ReceiveChannel {
    rx: Receiver<(String, String)>,
}

// endregion

// region:      EVENTS

struct RedrawEvent {
    width: f32,
    height: f32,
}

struct NewBoardEvent(String);

// endregion

fn main() {
    // get size of canvas object in HTML
    let canvas_elm = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("bevy-canvas")
        .unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas_elm
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    let canvas_width: f32 = canvas.client_width() as f32;
    let canvas_height: f32 = canvas.client_height() as f32;

    // construct sender
    let (tx, rx) = unbounded();
    unsafe {
        GLOBAL_SENDER = Some(Mutex::new(tx));
    }

    let receive_channel = ReceiveChannel { rx };

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
        .add_system(input_and_resizing_system)
        .add_event::<RedrawEvent>()
        .add_event::<NewBoardEvent>()
        .insert_resource(receive_channel)
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

fn input_and_resizing_system(
    buttons: Res<Input<MouseButton>>,
    touches: Res<Touches>,
    current_action: Res<CurrentAction>,
    mut windows: ResMut<Windows>,
    mut input_event_writer: EventWriter<InputEvent>,
    mut redraw_event_writer: EventWriter<RedrawEvent>,
    mut touch_event_reader: EventReader<TouchInput>,
) {
    let window = windows.get_primary().unwrap();

    // region:      Handle Resizing
    let canvas_elm = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("bevy-canvas")
        .unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas_elm
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    let canvas_width: f32 = canvas.client_width() as f32;
    let canvas_height: f32 = canvas.client_height() as f32;
    let window = windows.get_primary_mut().unwrap();

    if window.width() != canvas_width || window.height() != canvas_height {
        window.update_actual_size_from_backend(canvas_width as u32, canvas_height as u32);
        redraw_event_writer.send(RedrawEvent {
            width: canvas_width,
            height: canvas_height,
        });
    }
    // endregion

    // region:      Handle Input
    if let Some(screen_pos) = window.cursor_position() {
        if buttons.pressed(MouseButton::Left) {
            input_event_writer.send(InputEvent {
                x: screen_pos.x,
                y: screen_pos.y,
                action: current_action.0,
            });
        }
        if buttons.pressed(MouseButton::Right) {
            input_event_writer.send(InputEvent {
                x: screen_pos.x,
                y: screen_pos.y,
                action: BoardAction::Cross,
            });
        }

        for touch_event in touch_event_reader.iter() {
            match touch_event.phase {
                TouchPhase::Started => input_event_writer.send(InputEvent {
                    x: touch_event.position.x,
                    y: touch_event.position.y,
                    action: current_action.0,
                }),
                TouchPhase::Moved => {}
                TouchPhase::Ended => {}
                TouchPhase::Cancelled => {}
            }
        }
    }
    // endregion
}

fn receive_channel_system(
    receive_channel: Res<ReceiveChannel>,
    mut new_board_event_writer: EventWriter<NewBoardEvent>,
) {
    if let Ok(string) = receive_channel.rx.try_recv() {
        let command: &str = string.0.as_str();
        match command {
            // new board
            "n" => {
                new_board_event_writer.send(NewBoardEvent { 0: string.1 });
            }
            // unknown command
            _ => {}
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

/*
for mouse_event in mouse_button_event_reader.iter() {
           match mouse_event.button {
               MouseButton::Left => match mouse_event.state {
                   ButtonState::Pressed => {
                       input_event_writer.send(InputEvent {
                           x: screen_pos.x,
                           y: screen_pos.y,
                           action: current_action.0,
                       });
                   }
                   ButtonState::Released => {}
               },
               MouseButton::Right => match mouse_event.state {
                   ButtonState::Pressed => {
                       if let Some(screen_pos) = window.cursor_position() {
                           input_event_writer.send(InputEvent {
                               x: screen_pos.x,
                               y: screen_pos.y,
                               action: BoardAction::Cross,
                           });
                       }
                   }
                   ButtonState::Released => {}
               },
               _ => match mouse_event.state {
                   ButtonState::Pressed => {
                       if let Some(screen_pos) = window.cursor_position() {
                           input_event_writer.send(InputEvent {
                               x: screen_pos.x,
                               y: screen_pos.y,
                               action: BoardAction::Empty,
                           });
                       }
                   }
                   ButtonState::Released => {}
               },
           }
       }*/
