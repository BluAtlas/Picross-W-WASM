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

use crate::{GameTextures, NewBoardEvent, RedrawEvent, WinSize, SPRITE_SCALE, TILE_SIZE};

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
}

// endregion

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnTilesEvent>()
            .add_event::<DeleteTilesEvent>()
            .add_event::<DeletedTilesEvent>()
            .add_event::<InputEvent>()
            .add_startup_system_to_stage(StartupStage::PostStartup, startup_system)
            .add_system(spawn_tiles_event_system)
            .add_system(delete_tiles_event_system)
            .add_system(input_event_system)
            .add_system(redraw_event_system)
            .add_system(new_board_event_system);
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

    let current_action = CurrentAction(BoardAction::Fill);
    commands.insert_resource(current_action);

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

        deleted_tiles_event_writer.send(DeletedTilesEvent);
    }
}

fn spawn_tiles_event_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    mut spawn_tiles_event_reader: EventReader<SpawnTilesEvent>,
    board: Res<Board>,
) {
    for _ in spawn_tiles_event_reader.iter() {
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
                        texture: texture,
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

fn input_event_system(
    game_textures: Res<GameTextures>,
    mut board: ResMut<Board>,
    mut input_event_reader: EventReader<InputEvent>,
    mut tile_query: Query<(&mut Handle<Image>, &Tile)>,
    mut clue_query: Query<(&mut Text, &Clue)>,
) {
    for event in input_event_reader.iter() {
        // convert cursor position to tile coordinates
        let mut pos = Vec2::new(event.x, event.y);
        pos = pos - Vec2::new(board.origin.0, board.origin.1);
        pos = pos / board.pixels_per_tile;
        let x = pos.x.floor();
        let y = pos.y.floor();

        if x < board.p.get_longest_row_clue_len() as f32 && y >= board.h as f32 {
            // TODO switch between cross and fill modes here in the future
        } else if x < board.p.get_longest_row_clue_len() as f32 || y >= board.p.get_height() as f32
        {
            for (mut text, clue) in clue_query.iter_mut() {
                if clue.x == x && clue.y == y {
                    match event.action {
                        BoardAction::Fill => {
                            if text.sections[0].style.color == Color::RED {
                                text.sections[0].style.color = Color::BLACK;
                            } else {
                                text.sections[0].style.color = Color::RED;
                            }
                        }
                        BoardAction::Cross => {
                            if text.sections[0].style.color == Color::GRAY {
                                text.sections[0].style.color = Color::BLACK;
                            } else {
                                text.sections[0].style.color = Color::GRAY;
                            }
                        }
                        BoardAction::Empty => {
                            text.sections[0].style.color = Color::BLACK;
                        }
                    }
                }
            }
        } else {
            for (mut texture, tile) in tile_query.iter_mut() {
                if tile.x == x && tile.y == y {
                    let texture_copy = texture.clone();
                    // closure to set board and texture easier
                    let mut set_cell = |x: f32, y: f32, cell: Cell, t: Handle<Image>| {
                        let x_diff = board.p.get_longest_row_clue_len();
                        *texture = t;
                        board.p.set_cell(x as usize - x_diff, y as usize, cell)
                    };

                    match event.action {
                        BoardAction::Fill => {
                            if texture_copy == game_textures.tile_filled {
                                set_cell(x, y, Cell::Empty, game_textures.tile_empty.clone());
                            } else {
                                set_cell(x, y, Cell::Filled, game_textures.tile_filled.clone());
                            }
                        }
                        BoardAction::Cross => {
                            if texture_copy == game_textures.tile_crossed {
                                set_cell(x, y, Cell::Empty, game_textures.tile_empty.clone());
                            } else {
                                set_cell(x, y, Cell::Crossed, game_textures.tile_crossed.clone());
                            }
                        }
                        BoardAction::Empty => {
                            set_cell(x, y, Cell::Empty, game_textures.tile_empty.clone());
                        }
                    }
                }
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
        if let Ok(new_p) = Puzzle::from_string(event.0.as_str()) {
            board.p = new_p;
            resize_board_struct(board.as_mut(), win_size.as_ref());
            redraw_event_writer.send(RedrawEvent {
                width: win_size.w,
                height: win_size.h,
            })
        }
    }
}
