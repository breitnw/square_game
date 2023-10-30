use std::collections::HashMap;
use std::fmt::Debug;
use std::{env, fs, iter};
use std::path::Path;
use std::time::Duration;
use args::Args;
use rand::prelude::SliceRandom;
use rand::Rng;
use rust_embed::RustEmbed;
use sdl2;
use sdl2::event::{Event, WindowEvent};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, RenderTarget, Texture, TextureCreator, WindowCanvas};
use sdl2::image::LoadTexture;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseState;
use sdl2::rwops::RWops;
use sdl2::ttf::Font;
use sdl2::video::WindowContext;
use getopts;
use sdl2::sys;

#[cfg(test)]
mod tests;

// Take binary representation of each column
// Add each bit in 1s column, ignoring overflow
// Do the same for 2s, 4s, etc.
// At end of turn, each column should have a sum of 0 (or sum(10) modulo 2 is 0)
// In other words, the sum without any carrying (bitwise xor ^) should be 00000000...

#[derive(Copy, Clone, PartialEq, Debug)]
struct Meal {
    row_y: usize,
    amount: u8,
}

struct Board {
    filename: Option<String>,
    rows: Vec<Row>,
    current_turn: Player,
    auto: bool
}

impl Board {
    /// Creates a board with a set of defined row lengths.
    fn new(filename: Option<String>, row_lengths: Vec<u8>, starting_turn: Player, auto: bool) -> Self {
        let rows = row_lengths
            .iter()
            .map(|l| Row {orig_length: *l, eaten_squares: Vec::new()})
            .collect();
        Board {filename, rows, current_turn: starting_turn, auto }
    }

    fn from_file(filename: String, starting_turn: Player, auto: bool) -> Self {
        let row_lengths = fs::read_to_string(&filename)
            .unwrap()
            .lines()
            .filter_map(|x| x.parse().ok())
            .collect();
        Board::new(Some(filename), row_lengths, starting_turn, auto)
    }

    /// Creates a random board with `num_rows` rows, each with one to `max_row_length` squares.
    fn random(num_rows: usize, max_row_length: u8, starting_turn: Player, auto: bool) -> Self {
        let mut rng = rand::thread_rng();
        let rows = (0..num_rows)
            .map(|_| Row {orig_length: rng.gen_range(1..=max_row_length), eaten_squares: Vec::new()})
            .collect();
        Board { filename: None, rows, current_turn: starting_turn, auto }
    }

    /// Consumes a given number of squares from a given row, as defined by `meal`.
    fn eat(&mut self, meal: Meal) {
        self.rows.get_mut(meal.row_y).unwrap().eaten_squares
            .extend(iter::repeat(self.current_turn).take(meal.amount as usize));
    }

    /// Determines if the board will be optimal with test_amount removed from test_row; in other
    /// words, if the move guarantees that the player who takes it can win.
    ///
    /// This is done by folding the bitwise XOR of all of the rows (accounting for the modification)
    /// and checking if it equals 0. This operation is equivalent to taking the binary sum (ignoring
    /// overflow) of all of the bits in each column (1s, 2s, 4s, etc.) and checking if each sum is
    /// 0; which, in turn, is equivalent to taking the decimal sum of all of the bits in each column
    /// and checking if each sum mod 2 is 0.
    fn test_optimal(&self, test_meal: Meal) -> bool {
        self.rows
            .iter()
            .enumerate()
            .map(|(row_y, row)|
                row.get_remaining() - if row_y == test_meal.row_y { test_meal.amount } else { 0 })
            .fold(0, |acc, test_remaining| acc ^ test_remaining) == 0
    }

    /// Finds a move that makes the board optimal, if there is one. If such a move exists, returns
    /// an option containing a row and a number of squares to eat from that row; otherwise,
    /// returns None.
    fn find_optimal_move(&self) -> Option<Meal> {
        for (row_y, row) in self.rows.iter().enumerate() {
            for amount in 1..=row.get_remaining() {
                if self.test_optimal(Meal { row_y, amount }) {
                    return Some(Meal {row_y, amount})
                }
            }
        }
        None
    }

    /// Returns a move with one square and a random (available) row. Returns None if there are no
    /// rows available.
    fn find_random_move(&self) -> Option<Meal> {
        let available_rows = self.rows.iter()
            .enumerate()
            .filter(|(_, row)| row.get_remaining() > 0)
            .map(|(row_y, _)| row_y)
            .collect::<Vec<usize>>();
        let row_y = available_rows
            .choose(&mut rand::thread_rng())
            .clone();

        if let Some(row_y) = row_y {
            Some(Meal {row_y: *row_y, amount: 1})
        } else {
            None
        }
    }

    /// Performs an optimal move if one exists, or an arbitrary move if one does not. Panic!s if
    /// there is no move available.
    fn take_optimal_move(&mut self) {
        if let Some(meal) = self.find_optimal_move() {
            self.eat(meal)
        } else {
            let meal = self.find_random_move().unwrap();
            self.eat(meal)
        }
    }

    /// Switches the current turn from red to blue or vice versa
    fn next_turn (&mut self) {
        self.current_turn = match self.current_turn {
            Player::RED => Player::BLUE,
            Player::BLUE => Player::RED,
        }
    }

    /// Determines if the board is empty; i.e., all of the squares have been eaten
    fn is_empty(&self) -> bool {
        self.rows.iter()
            .filter(|r| r.eaten_squares.len() != r.orig_length as usize)
            .count() == 0
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Player {
    RED,
    BLUE,
}

struct Row {
    pub orig_length: u8,
    pub eaten_squares: Vec<Player>
}

impl Row {
    fn get_remaining(&self) -> u8{
        self.orig_length - self.eaten_squares.len() as u8
    }
}

#[derive(RustEmbed)]
#[folder = "assets/images/"]
struct TextureAsset;

fn load_textures<'a, T: 'a>(texture_creator: &'a TextureCreator<T>) -> HashMap<String, Texture<'a>> {
    TextureAsset::iter()
        .map(|file_path| {
            let tex = texture_creator
                .load_texture_bytes(&TextureAsset::get(&file_path).unwrap().data)
                .unwrap_or_else(|_| panic!("Unable to load {}", &file_path));
            (file_path.to_string().strip_suffix(".png").unwrap().to_string(), tex)
        })
        .collect()
}

const SQUARE_SIZE: u32 = 32;
const SQUARE_SPACING: u32 = 4;
const SQUARE_OFFSET: u32 = 8;
const SHADOW_OFFSET: i32 = 3;

fn get_square_rect(sqr_x: u8, row_y: usize) -> Rect {
    Rect::new((sqr_x as u32 * (SQUARE_SPACING + SQUARE_SIZE) + SQUARE_OFFSET) as i32,
              (row_y as u32 * (SQUARE_SPACING + SQUARE_SIZE) + SQUARE_OFFSET) as i32, SQUARE_SIZE, SQUARE_SIZE)
}

fn draw(canvas: &mut WindowCanvas,
        board: &Board,
        textures: &mut HashMap<String, Texture>,
        mouse_state: MouseState,
        font: &Font,
        texture_creator: &TextureCreator<WindowContext>) {
    for (row_y, row) in board.rows.iter().enumerate() {
        for sqr_x in 0..row.orig_length {
            let square = get_square_rect(sqr_x, row_y);
            let shadow = Rect::new(square.x + SHADOW_OFFSET, square.y + SHADOW_OFFSET, square.w as u32, square.h as u32);

            canvas.set_draw_color(Color::RGBA(0, 0, 0, 50));
            canvas.set_blend_mode(BlendMode::Mul);
            canvas.fill_rect(shadow).unwrap();
            canvas.set_blend_mode(BlendMode::None);
            canvas.copy(&textures["square"], None, square).unwrap();

            if mouse_state.y() < square.bottom() && mouse_state.y() > square.top() && mouse_state.x() < square.right() {
                let id = match board.current_turn {
                    Player::BLUE => "x-blue",
                    Player::RED => "x-red",
                };
                let tex = textures.get_mut(id).unwrap();
                tex.set_alpha_mod(100);
                canvas.copy(tex, None, square).unwrap();
                tex.set_alpha_mod(255);
            }
            if let Some(player) = row.eaten_squares.get((row.orig_length - sqr_x - 1) as usize) {
                let id = match player {
                    Player::BLUE => "x-blue",
                    Player::RED => "x-red",
                };
                canvas.copy(&textures[id], None, square).unwrap();
            }

            // if we're on the last square, draw the count in binary afterward
            if sqr_x == row.orig_length - 1 {
                let binary = format!("{:08b}", row.get_remaining());
                let num_significant = binary.chars().skip_while(|c| *c == '0').count();
                let text = font.render(
                    &binary.chars()
                        .skip(if num_significant > 4 {0} else {4})
                        .collect::<String>())
                    .solid(Color::RGB(234, 200, 102)).unwrap();
                let mut text_rect = text.rect();

                text_rect.set_x(square.right() + 6);
                text_rect.set_y(square.y() + 6);

                let text_texture = texture_creator.create_texture_from_surface(text).unwrap();
                canvas.copy(&text_texture, None, text_rect).unwrap();
            }
        }
    }
}
fn update_canvas_scale<T: RenderTarget>(
    canvas: &mut Canvas<T>,
    window_width: u32,
    window_height: u32,
) {
    let (w, h) = canvas.output_size().unwrap();
    canvas
        .set_scale((w / window_width) as f32, (h / window_height) as f32)
        .unwrap();
}

unsafe fn parse_args() -> Board {
    let mut args = Args::new("square_game", "an optimal square game solver created for NEU CS1800 Acc.");
    args.flag("h", "help", "Print the usage menu");
    args.flag("a", "auto", "Pit two bots against each other instead of playing manually");
    args.option("f",
                "file",
                "The path to a file containing a test case for the program",
                "FILENAME",
                getopts::Occur::Optional,
                None);
    args.parse(env::args().collect::<Vec<_>>()).unwrap_or_else(|a| {
        println!("Error: Arguments not in correct format");
        println!("{}", args.full_usage());
        sys::exit(1);
    });

    let help = args.value_of("help")
        .is_ok_and(|x: String| x == "true");
    let auto = args.value_of("auto")
        .is_ok_and(|x: String| x == "true");
    let filename: Option<String> = args.value_of("file")
        .ok()
        .and_then(|f|
            if f == "false" { None }
            else if !Path::exists(Path::new(&f)) {
                println!("Error: No file exists: {}", f);
                sys::exit(1);
                None
            } else { Some(f) } );

    if help {
        println!("{}", args.full_usage());
        sys::exit(0);
    }

    if let Some(filename) = filename {
        Board::from_file(filename, Player::RED, auto)
    } else {
        Board::random(6, 10, Player::RED, auto)
    }
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    const WINDOW_WIDTH: u32 = 700;
    const WINDOW_HEIGHT: u32 = 300;

    // init window and canvas
    let window = video_subsystem.window("square game", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .allow_highdpi()
        .build()
        .unwrap();
    let mut canvas = window
        .into_canvas()
        .build()
        .unwrap();

    // init textures
    let texture_creator = canvas.texture_creator();
    let mut textures = load_textures(&texture_creator);

    // prepare the canvas
    canvas.set_draw_color(Color::RGB(0,90,20));
    update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT);
    canvas.clear();
    canvas.present();

    // load the font
    let ttf_context = sdl2::ttf::init().unwrap();
    let font_ttf = RWops::from_bytes(include_bytes!("../assets/pixel-bit-advanced.ttf")).unwrap();
    let font = ttf_context.load_font_from_rwops(font_ttf, 16).unwrap();

    // prepare the game state
    let mut board = unsafe { parse_args() };

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        canvas.set_draw_color(Color::RGB(0,90,20));
        canvas.clear();

        let mouse_state = MouseState::new(&event_pump);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} => { break 'running }
                Event::MouseButtonDown {..} => {
                    for row_y in 0..board.rows.len() {
                        let eat_to: i32 = (0..board.rows[row_y].orig_length).filter(|sqr_x| {
                            let square = get_square_rect(*sqr_x, row_y);
                            mouse_state.y() < square.bottom() && mouse_state.y() > square.top() && mouse_state.x() < square.right()
                        }).collect::<Vec<u8>>().len() as i32;
                        let squares_eaten = board.rows[row_y].eaten_squares.len() as i32;
                        let squares_to_eat = eat_to - squares_eaten;
                        if squares_to_eat > 0 {
                            board.eat(Meal { row_y, amount: squares_to_eat as u8 });
                            board.next_turn();
                            break;
                        }
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    if let Some(filename) = board.filename {
                        board = Board::from_file(filename, Player::RED, board.auto)
                    } else {
                        board = Board::random(6, 15, Player::RED, board.auto);
                    }
                }
                Event::Window { win_event, .. } => {
                    match win_event {
                        WindowEvent::Moved { .. } => {
                            // Update the canvas scale in case the user drags the window to a different monitor
                            update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if (board.auto || board.current_turn == Player::BLUE) && !board.is_empty() {
            board.take_optimal_move();
            board.next_turn();
        }

        // Draw the board
        draw(&mut canvas, &board, &mut textures, mouse_state, &font, &texture_creator);

        // Draw victory text if there are no squares left
        if board.is_empty() {
            let text = match board.current_turn {
                Player::BLUE => { font.render("red wins!").solid(Color::RGB(202, 95, 102)).unwrap() }
                Player::RED => { font.render("blue wins!").solid(Color::RGB(112, 154, 248)).unwrap() }
            };
            let mut text_rect = text.rect();
            text_rect.set_bottom(canvas.viewport().bottom() - 6);
            text_rect.set_x(6);
            let text_texture = &texture_creator.create_texture_from_surface(text).unwrap();
            canvas.copy(text_texture, None, text_rect).unwrap();
        }

        // Draw help text instructing the player how to restart
        let text = font.render("restart (r)").solid(Color::RGB(0, 50, 8)).unwrap();
        let mut text_rect = text.rect();
        text_rect.set_bottom(canvas.viewport().bottom() - 6);
        text_rect.set_right(canvas.viewport().right() - 6);
        let text_texture = &texture_creator.create_texture_from_surface(text).unwrap();
        canvas.copy(text_texture, None, text_rect).unwrap();

        update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT);
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60))
    }
}