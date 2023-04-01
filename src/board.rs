// region:      IMPORTS

use bevy::{
    ecs::component,
    input::{mouse::MouseButtonInput, touch::TouchPhase, ButtonState},
    prelude::{system_adapter::new, *},
    render::render_resource::Texture,
    sprite::Anchor,
};
use picross_handler::{Cell, Puzzle};
use wasm_bindgen::prelude::*;

use crate::{
    BoardUpdateEvent, GameTextures, NewBoardEvent, WASMSendChannel, WinSize, SPRITE_SCALE,
    TILE_SIZE,
};

// endregion

// region:      CONSTANTS

const TILE_Z: f32 = 1.;
const CLUE_Z: f32 = 2.;

// endregion

// region:      COMPONENTS

#[derive(Component)]
pub struct Tile {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Clue {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct ControlTile {
    pub x: f32,
    pub y: f32,
}

// endregion

// region:      RESOURCES

#[derive(Resource)]
struct Board {
    pub tile_scale: f32,
    pub pixels_per_tile: f32,
    pub origin: (f32, f32),
    pub h: usize,
    pub w: usize,
    pub p: Puzzle,
}

#[derive(Clone, Copy)]
pub enum BoardAction {
    Fill,
    Cross,
    Empty,
}

// touchscreen helper resource

#[derive(Resource)]
pub struct CurrentAction(pub BoardAction);

#[derive(Resource)]
pub struct ControlAction(pub BoardAction);

impl Default for Board {
    fn default() -> Self {
        Self {
            tile_scale: Default::default(),
            pixels_per_tile: Default::default(),
            origin: Default::default(),
            h: Default::default(),
            w: Default::default(),
            p: Default::default(),
        }
    }
}

// endregion

// region:      EVENTS

pub struct SpawnTilesEvent;
pub struct DeleteTilesEvent;
pub struct DeletedTilesEvent;

pub struct InputEvent {
    pub x: f32,
    pub y: f32,
    pub action: BoardAction,
    pub from_player: bool,
}

struct RedrawEvent {
    width: f32,
    height: f32,
}

// endregion

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnTilesEvent>()
            .add_event::<DeleteTilesEvent>()
            .add_event::<DeletedTilesEvent>()
            .add_event::<InputEvent>()
            .add_event::<RedrawEvent>()
            .add_startup_system_to_stage(StartupStage::PostStartup, startup_system)
            .add_system(spawn_tiles_event_system)
            .add_system(delete_tiles_event_system)
            .add_system(input_event_system)
            .add_system(redraw_event_system)
            .add_system(input_and_resizing_system)
            .add_system(new_board_event_system)
            .add_system(board_update_event_system);
    }
}

fn startup_system(
    mut commands: Commands,
    win_size: Res<WinSize>,
    mut spawn_tiles_event_writer: EventWriter<SpawnTilesEvent>,
) {
    // create resources
    let mut board = Board {
        ..Default::default()
    };

    resize_board_struct(&mut board, win_size.as_ref());
    commands.insert_resource(board);

    commands.insert_resource(CurrentAction(BoardAction::Fill));

    commands.insert_resource(ControlAction(BoardAction::Fill));

    spawn_tiles_event_writer.send(SpawnTilesEvent);
}

fn redraw_event_system(
    mut redraw_event_reader: EventReader<RedrawEvent>,
    mut deleted_tiles_event_reader: EventReader<DeletedTilesEvent>,
    mut spawn_tiles_event_writer: EventWriter<SpawnTilesEvent>,
    mut delete_tiles_event_writer: EventWriter<DeleteTilesEvent>,
    mut win_size: ResMut<WinSize>,
    mut board: ResMut<Board>,
) {
    for event in redraw_event_reader.iter() {
        win_size.w = event.width;
        win_size.h = event.height;
        delete_tiles_event_writer.send(DeleteTilesEvent);
    }

    for event in deleted_tiles_event_reader.iter() {
        resize_board_struct(board.as_mut(), win_size.as_ref());
        spawn_tiles_event_writer.send(SpawnTilesEvent);
    }
}

fn resize_board_struct(mut board: &mut Board, win_size: &WinSize) {
    // init board variables
    let mut origin: (f32, f32);
    let window_pixel_distance;
    let tile_size;
    let pixels_per_tile;
    let tile_scale;

    let total_board_width = (board.p.get_width() + board.p.get_longest_row_clue_len());
    let total_board_height = (board.p.get_height() + board.p.get_longest_column_clue_len());

    // account for aspect ratio
    if win_size.w < win_size.h {
        window_pixel_distance = win_size.w;
        tile_size = TILE_SIZE.0;
        pixels_per_tile = window_pixel_distance / total_board_width as f32;
        tile_scale = pixels_per_tile / tile_size;
        origin = (
            0.,
            (win_size.h - (total_board_height as f32 * pixels_per_tile)) / 2.,
        );
    } else {
        window_pixel_distance = win_size.h;
        tile_size = TILE_SIZE.1;
        pixels_per_tile = window_pixel_distance / total_board_height as f32;
        tile_scale = pixels_per_tile / tile_size;
        origin = (
            (win_size.w - (total_board_width as f32 * pixels_per_tile)) / 2.,
            0.,
        );
    }

    board.origin = origin;
    board.tile_scale = tile_scale;
    board.pixels_per_tile = pixels_per_tile;
    board.w = total_board_width;
    board.h = total_board_height;
}

fn delete_tiles_event_system(
    mut commands: Commands,
    mut tile_query: Query<Entity, With<Tile>>,
    mut clue_query: Query<Entity, With<Clue>>,
    mut control_tile_query: Query<Entity, With<ControlTile>>,
    mut delete_tiles_event_reader: EventReader<DeleteTilesEvent>,
    mut deleted_tiles_event_writer: EventWriter<DeletedTilesEvent>,
) {
    for _ in delete_tiles_event_reader.iter() {
        for entity in tile_query.iter_mut() {
            commands.entity(entity).despawn();
        }

        for entity in clue_query.iter_mut() {
            commands.entity(entity).despawn();
        }

        for entity in control_tile_query.iter_mut() {
            commands.entity(entity).despawn();
        }

        deleted_tiles_event_writer.send(DeletedTilesEvent);
    }
}

fn spawn_tiles_event_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    mut spawn_tiles_event_reader: EventReader<SpawnTilesEvent>,
    board: Res<Board>,
    control_action: Res<ControlAction>,
) {
    for _ in spawn_tiles_event_reader.iter() {
        let control_tile_max_size;
        if (board.p.get_longest_column_clue_len() < board.p.get_longest_row_clue_len()) {
            control_tile_max_size = board.p.get_longest_column_clue_len();
        } else {
            control_tile_max_size = board.p.get_longest_row_clue_len();
        }
        // spawn ControlTile sprite
        commands
            .spawn(SpriteBundle {
                texture: match control_action.0 {
                    BoardAction::Fill => game_textures.tile_filled.clone(),
                    BoardAction::Cross => game_textures.tile_crossed.clone(),
                    BoardAction::Empty => game_textures.tile_empty.clone(),
                },
                sprite: Sprite {
                    anchor: Anchor::Center,
                    ..Default::default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        board.origin.0
                            + board.p.get_longest_row_clue_len() as f32 * board.pixels_per_tile
                                / 2.,
                        board.origin.1
                            + board.p.get_height() as f32 * board.pixels_per_tile
                            + board.p.get_longest_column_clue_len() as f32 * board.pixels_per_tile
                                / 2.,
                        TILE_Z,
                    ),
                    scale: Vec3::new(
                        board.tile_scale * control_tile_max_size as f32 * 0.8,
                        board.tile_scale * control_tile_max_size as f32 * 0.8,
                        1.,
                    ),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(ControlTile {
                x: board.origin.0
                    + board.p.get_longest_row_clue_len() as f32 * board.pixels_per_tile / 2.,
                y: board.origin.1
                    + board.p.get_height() as f32 * board.pixels_per_tile
                    + board.p.get_longest_column_clue_len() as f32 * board.pixels_per_tile / 2.,
            });

        // create tiles
        for x in (0..board.w as usize) {
            for y in (0..board.h as usize) {
                // set texture
                let texture;
                if (x >= board.p.get_longest_row_clue_len()
                    && y < board.h - board.p.get_longest_column_clue_len())
                {
                    // if tile is not a clue tile
                    let x = x - board.p.get_longest_row_clue_len();
                    let y = y;
                    if (board.p.get_cell(x, y) == Cell::Filled) {
                        texture = game_textures.tile_filled.clone();
                    } else if board.p.get_cell(x, y) == Cell::Crossed {
                        texture = game_textures.tile_crossed.clone();
                    } else {
                        texture = game_textures.tile_empty.clone();
                    }
                } else if x >= board.p.get_longest_row_clue_len()
                    || y < board.h - board.p.get_longest_column_clue_len()
                {
                    // else if tile is a clue tile
                    texture = game_textures.tile_clue.clone();
                    // set the clue value, if applicable
                    let mut clue_str = String::from("");
                    if x >= board.p.get_longest_row_clue_len() {
                        // column clue
                        if board.p.column_clues[x - board.p.get_longest_row_clue_len()].len()
                            > y - (board.p.get_height())
                        {
                            let y = y - board.p.get_height();
                            let x = x - board.p.get_longest_row_clue_len();
                            clue_str = board.p.column_clues[x]
                                [(0 + board.p.column_clues[x].len() - 1) - y]
                                .to_string();
                        };
                    } else if (board.p.get_longest_row_clue_len() - board.p.row_clues[y].len()) <= x
                    {
                        // row clue
                        clue_str = board.p.row_clues[y]
                            [x - (board.p.get_longest_row_clue_len() - board.p.row_clues[y].len())]
                        .to_string();
                    }
                    // spawn text for clue
                    commands
                        .spawn(Text2dBundle {
                            text: Text::from_section(
                                clue_str,
                                TextStyle {
                                    font: game_textures.font.clone(),
                                    font_size: TILE_SIZE.0 * 0.5 * board.tile_scale,
                                    color: Color::BLACK,
                                },
                            )
                            .with_alignment(TextAlignment::CENTER),
                            transform: Transform {
                                translation: Vec3::new(
                                    board.origin.0
                                        + x as f32 * board.pixels_per_tile
                                        + board.pixels_per_tile / 2.,
                                    board.origin.1
                                        + y as f32 * board.pixels_per_tile
                                        + board.pixels_per_tile / 2.,
                                    CLUE_Z,
                                ),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(Clue {
                            x: x as f32,
                            y: y as f32,
                        });
                } else {
                    // else not a tile, continue
                    continue;
                }

                // spawn tile sprite
                commands
                    .spawn(SpriteBundle {
                        texture,
                        sprite: Sprite {
                            anchor: Anchor::BottomLeft,
                            ..Default::default()
                        },
                        transform: Transform {
                            translation: Vec3::new(
                                board.origin.0 + x as f32 * board.pixels_per_tile,
                                board.origin.1 + y as f32 * board.pixels_per_tile,
                                TILE_Z,
                            ),

                            scale: Vec3::new(board.tile_scale, board.tile_scale, 1.),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .insert(Tile {
                        x: x as f32,
                        y: y as f32,
                    });
            }
        }
    }
}

fn input_and_resizing_system(
    buttons: Res<Input<MouseButton>>,
    touches: Res<Touches>,
    board: Res<Board>,
    game_textures: Res<GameTextures>,
    mut current_action: ResMut<CurrentAction>,
    mut control_action: ResMut<ControlAction>,
    mut windows: ResMut<Windows>,
    mut input_event_writer: EventWriter<InputEvent>,
    mut redraw_event_writer: EventWriter<RedrawEvent>,
    mut tile_query: Query<(&mut Handle<Image>, &Tile)>,
    mut clue_query: Query<(&mut Text, &Clue)>,
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
        info!("size changed");
        window.update_actual_size_from_backend(canvas_width as u32, canvas_height as u32);
        redraw_event_writer.send(RedrawEvent {
            width: canvas_width,
            height: canvas_height,
        });
    }
    // endregion

    // region:      Handle Input
    if let Some(screen_pos) = window.cursor_position() {
        // convert screen coordinates to board coordinates
        let mut pos = Vec2::new(screen_pos.x, screen_pos.y);
        pos = pos - Vec2::new(board.origin.0, board.origin.1);
        pos = pos / board.pixels_per_tile;
        let x = pos.x.floor();
        let y = pos.y.floor();

        // region: Mouse Input

        if buttons.just_pressed(MouseButton::Left) {
            current_action.0 = control_action.0;
        } else if buttons.just_pressed(MouseButton::Right) {
            current_action.0 = BoardAction::Cross;
        } else if buttons.just_pressed(MouseButton::Middle) {
            current_action.0 = BoardAction::Empty;
        }

        // account for cases where the action already matches the current state of object under cursor
        if buttons.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
            if x < board.p.get_longest_row_clue_len() as f32 && y >= board.p.get_height() as f32 {
                input_event_writer.send(InputEvent {
                    x,
                    y,
                    action: current_action.0,
                    from_player: true,
                });
            } else if x < board.p.get_longest_row_clue_len() as f32
                || y >= board.p.get_height() as f32
            {
                // account for clues matching action here
                let red = Color::RED;
                let gray = Color::GRAY;
                for (mut text, clue) in clue_query.iter_mut() {
                    if clue.x == x && clue.y == y {
                        match (current_action.0) {
                            (BoardAction::Fill) => {
                                if text.sections[0].style.color == Color::RED {
                                    current_action.0 = BoardAction::Empty;
                                }
                            }
                            (BoardAction::Cross) => {
                                if text.sections[0].style.color == Color::GRAY {
                                    current_action.0 = BoardAction::Empty;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                // account for tiles matching action here
                for (mut texture, tile) in tile_query.iter_mut() {
                    if tile.x == x && tile.y == y {
                        let texture_copy = texture.clone();
                        let filled = game_textures.tile_filled.clone();
                        let crossed = game_textures.tile_crossed.clone();
                        match (current_action.0) {
                            (BoardAction::Fill) => {
                                if texture_copy == filled {
                                    current_action.0 = BoardAction::Empty;
                                }
                            }
                            (BoardAction::Cross) => {
                                if texture_copy == crossed {
                                    current_action.0 = BoardAction::Empty;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        if buttons.any_pressed([MouseButton::Left, MouseButton::Right, MouseButton::Middle])
            && !(x < board.p.get_longest_row_clue_len() as f32 && y >= board.p.get_height() as f32)
        // && not in control tile
        {
            input_event_writer.send(InputEvent {
                x,
                y,
                action: current_action.0,
                from_player: true,
            });
        }

        // endregion
    }
    // endregion
}

fn input_event_system(
    game_textures: Res<GameTextures>,
    send_channel: Res<WASMSendChannel>,
    mut board: ResMut<Board>,
    mut input_event_reader: EventReader<InputEvent>,
    mut tile_query: Query<(&mut Handle<Image>, &Tile), Without<ControlTile>>,
    mut clue_query: Query<(&mut Text, &Clue)>,
    mut control_tile_query: Query<(&mut Handle<Image>), (With<ControlTile>, Without<Tile>)>,
    mut current_action: ResMut<CurrentAction>,
    mut control_action: ResMut<ControlAction>,
) {
    for event in input_event_reader.iter() {
        // convert cursor position to tile coordinates

        let x = event.x;
        let y = event.y;

        if x < board.p.get_longest_row_clue_len() as f32 && y >= board.p.get_height() as f32 {
            // switch between cross and fill modes here for touch
            control_action.0 = match control_action.0 {
                BoardAction::Fill => BoardAction::Cross,
                BoardAction::Cross => BoardAction::Fill,
                BoardAction::Empty => BoardAction::Fill,
            };
        } else if x < board.p.get_longest_row_clue_len() as f32 || y >= board.p.get_height() as f32
        // handle clues
        {
            for (mut text, clue) in clue_query.iter_mut() {
                if clue.x == x && clue.y == y {
                    match event.action {
                        BoardAction::Fill => {
                            text.sections[0].style.color = Color::RED;
                        }
                        BoardAction::Cross => {
                            text.sections[0].style.color = Color::GRAY;
                        }
                        BoardAction::Empty => {
                            text.sections[0].style.color = Color::BLACK;
                        }
                    }
                }
            }
        } else {
            // handle tiles
            for (mut texture, tile) in tile_query.iter_mut() {
                if tile.x == x && tile.y == y {
                    // closure to set board and texture easier
                    let mut set_cell = |x: f32, y: f32, cell: Cell, t: Handle<Image>| {
                        let x_diff = board.p.get_longest_row_clue_len();
                        let current_cell = board.p.get_cell(x as usize - x_diff, y as usize);
                        // update and send changes if cell is different, otherwise do nothing
                        if current_cell != cell {
                            *texture = t;
                            board.p.set_cell(x as usize - x_diff, y as usize, cell);
                            if event.from_player {
                                let cell_str;
                                match cell {
                                    Cell::Empty => cell_str = String::from("0"),
                                    Cell::Filled => cell_str = String::from("1"),
                                    Cell::Crossed => cell_str = String::from("X"),
                                }
                                // send update to server if the player made the action
                                send_channel.tx.send((
                                    String::from("c"),
                                    format!(
                                        "{},{}",
                                        board.p.get_pos(x as usize - x_diff, y as usize),
                                        cell_str
                                    ),
                                ));
                            }
                        }
                    };

                    match event.action {
                        BoardAction::Fill => {
                            set_cell(x, y, Cell::Filled, game_textures.tile_filled.clone());
                        }
                        BoardAction::Cross => {
                            set_cell(x, y, Cell::Crossed, game_textures.tile_crossed.clone());
                        }
                        BoardAction::Empty => {
                            set_cell(x, y, Cell::Empty, game_textures.tile_empty.clone());
                        }
                    }
                }
            }
        }
        // update control tile
        for (mut texture) in control_tile_query.iter_mut() {
            match control_action.0 {
                BoardAction::Fill => *texture = game_textures.tile_filled.clone(),
                BoardAction::Cross => *texture = game_textures.tile_crossed.clone(),
                BoardAction::Empty => *texture = game_textures.tile_empty.clone(),
            }
        }
    }
}

fn new_board_event_system(
    win_size: Res<WinSize>,
    mut board: ResMut<Board>,
    mut redraw_event_writer: EventWriter<RedrawEvent>,
    mut new_board_event_reader: EventReader<NewBoardEvent>,
) {
    for event in new_board_event_reader.iter() {
        match Puzzle::from_string(event.clues.as_str()) {
            Ok(mut new_p) => {
                new_p.set_board_from_string(event.cells.as_str());

                board.p = new_p;
                resize_board_struct(board.as_mut(), win_size.as_ref());
                redraw_event_writer.send(RedrawEvent {
                    width: win_size.w,
                    height: win_size.h,
                })
            }
            Err(err) => warn!(err),
        }
    }
}

fn board_update_event_system(
    board: Res<Board>,
    mut input_event_writer: EventWriter<InputEvent>,
    mut board_update_event_reader: EventReader<BoardUpdateEvent>,
) {
    for event in board_update_event_reader.iter() {
        let cells = &event.0;
        let mut cells_iter = cells.chars().into_iter();

        if (board.p.get_width() * board.p.get_height() == cells.chars().count()) {
            for y in 0..board.p.get_height() {
                for x in 0..board.p.get_width() {
                    let cell = cells_iter.next();
                    let mut input_event = InputEvent {
                        x: (x + board.p.get_longest_row_clue_len()) as f32,
                        y: y as f32,
                        action: BoardAction::Empty,
                        from_player: false,
                    };
                    match cell {
                        Some('0') => {
                            input_event.action = BoardAction::Empty;
                            input_event_writer.send(input_event);
                        }
                        Some('1') => {
                            input_event.action = BoardAction::Fill;
                            input_event_writer.send(input_event);
                        }
                        Some('X') => {
                            input_event.action = BoardAction::Cross;
                            input_event_writer.send(input_event);
                        }
                        Some(c) => {
                            warn!("Invalid BoardUpdateEvent, Incorrect Cell: {}", c)
                        }
                        None => {
                            warn!("Invalid BoardUpdateEvent, Ran out of input") // should never happen
                        }
                    }
                }
            }
        } else {
            warn!(
                "Invalid BoardUpdateEvent, Incorrect size: {}, Expected: {}",
                cells.chars().count(),
                board.p.get_width() * board.p.get_height()
            )
        }
    }
}
