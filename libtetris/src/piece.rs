use enumset::{ EnumSet, EnumSetType, enum_set };
use enum_map::Enum;
use serde::{ Serialize, Deserialize };

use crate::{ Board, Row };

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct FallingPiece {
    pub kind: PieceState,
    pub x: i32,
    pub y: i32,
    pub tspin: TspinStatus
}

impl FallingPiece {
    pub fn spawn<R: Row>(piece: Piece, board: &Board<R>) -> Option<FallingPiece> {
        let mut this = FallingPiece {
            kind: PieceState(piece, RotationState::North),
            x: 4, y: 20,
            tspin: TspinStatus::None
        };

        if board.obstructed(&this) {
            None
        } else {
            this.y -= 1;
            if board.obstructed(&this) {
                this.y += 1;
            }

            Some(this)
        }
    }

    #[inline]
    pub fn cells(&self) -> [(i32, i32); 4] {
        let mut cells = self.kind.cells();
        for (dx, dy) in cells.iter_mut() {
            *dx += self.x;
            *dy += self.y;
        }
        cells
    }

    #[inline]
    pub fn cells_with_connections(&self) -> [(i32, i32, EnumSet<Direction>); 4] {
        let mut cells = self.kind.cells_with_connections();
        for (dx, dy, _) in cells.iter_mut() {
            *dx += self.x;
            *dy += self.y;
        }
        cells
    }

    pub fn shift<R: Row>(&mut self, board: &Board<R>, dx: i32, dy: i32) -> bool {
        self.x += dx;
        self.y += dy;
        if board.obstructed(self) {
            self.x -= dx;
            self.y -= dy;
            false
        } else {
            self.tspin = TspinStatus::None;
            true
        }
    }

    pub fn sonic_drop<R: Row>(&mut self, board: &Board<R>) -> bool {
        let drop_by = self.cells()
            .iter()
            .map(|&(x, y)| y - board.column_heights()[x as usize])
            .min().unwrap();
        if drop_by > 0 {
            self.tspin = TspinStatus::None;
            self.y -= drop_by;
            true
        } else if drop_by < 0 {
            let mut fell = false;
            loop {
                self.y -= 1;
                if board.obstructed(self) {
                    self.y += 1;
                    break
                }
                fell = true;
                self.tspin = TspinStatus::None;
            }
            fell
        } else {
            false
        }
    }

    fn rotate<R: Row>(&mut self, target: PieceState, board: &Board<R>, is_ccw: bool) -> bool {
        let initial = *self;
        self.kind = target;
        let kicks = if is_ccw {
            [(0, 0), (1, 0), (0, -1), (1, -1), (0, -2), (1, -2), (2, 0), (2, -1), (2, -2), (-1, 0), (-1, -1), (0, 1), (1, 1), (2, 1), (-1, -2), (-2, 0), (0, 2), (1, 2), (2, 2), (-2, -1), (-2, -2), (-1, 1)]
        } else {
            [(0, 0), (-1, 0), (0, -1), (-1, -1), (0, -2), (-1, -2), (-2, 0), (-2, -1), (-2, -2), (1, 0), (1, -1), (0, 1), (-1, 1), (-2, 1), (1, -2), (2, 0), (0, 2), (-1, 2), (-2, 2), (2, -1), (2, -2), (1, 1)]
        };

        for &(dx, dy) in &kicks {
            self.x = initial.x + dx;
            self.y = initial.y + dy;
            if !board.obstructed(self) {
                let mut piece = *self;

                if !piece.shift(board, -1, 0) && !piece.shift(board, 1, 0) && !piece.shift(board, 0, 1) && !piece.shift(board, 0, -1) {
                    self.tspin = TspinStatus::Full;
                } else {
                    self.tspin = TspinStatus::None;
                }
                return true
            }
        }
        
        *self = initial;
        false
    }

    pub fn cw<R: Row>(&mut self, board: &Board<R>) -> bool {
        let mut target = self.kind;
        target.cw();
        self.rotate(target, board, false)
    }

    pub fn ccw<R: Row>(&mut self, board: &Board<R>) -> bool {
        let mut target = self.kind;
        target.ccw();
        self.rotate(target, board, true)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CellColor {
    I, O, T, L, J, S, Z,
    Garbage,
    Unclearable,
    Empty
}

#[derive(Debug, Hash, EnumSetType, Enum, Serialize, Deserialize)]
pub enum Piece {
    I, O, T, L, J, S, Z
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum RotationState {
    North, South, East, West
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PieceState(pub Piece, pub RotationState);

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TspinStatus {
    None,
    Full,
}

impl RotationState {
    pub fn cw(&mut self) {
        use RotationState::*;
        match self {
            North => *self = East,
            East  => *self = South,
            South => *self = West,
            West  => *self = North
        }
    }
    
    pub fn ccw(&mut self) {
        use RotationState::*;
        match self {
            North => *self = West,
            West  => *self = South,
            South => *self = East,
            East  => *self = North
        }
    }
}

impl PieceState {
    pub fn cw(&mut self) {
        self.1.cw()
    }

    pub fn ccw(&mut self) {
        self.1.ccw()
    }

    /// Returns the cells this piece and orientation occupy relative to rotation point 1, as well
    /// as the connection directions, in no particular order.
    #[inline]
    pub fn cells(&self) -> [(i32, i32); 4] {
        let rotate = |x: i32, y| match self.1 {
            RotationState::North => (x, y),
            RotationState::East => (y, -x),
            RotationState::South => (-x, -y),
            RotationState::West => (-y, x)
        };
        match self.0 {
            Piece::I => [rotate(-1, 0), rotate( 0, 0), rotate( 1, 0), rotate( 2, 0)],
            Piece::O => [rotate( 0, 0), rotate( 1, 0), rotate( 0, 1), rotate( 1, 1)],
            Piece::L => [rotate(-1, 0), rotate( 0, 0), rotate( 1, 0), rotate( 1, 1)],
            Piece::J => [rotate(-1, 0), rotate( 0, 0), rotate( 1, 0), rotate(-1, 1)],
            Piece::T => [rotate(-1, 0), rotate( 0, 0), rotate( 1, 0), rotate( 0, 1)],
            Piece::S => [rotate(-1, 0), rotate( 0, 0), rotate( 0, 1), rotate( 1, 1)],
            Piece::Z => [rotate(-1, 1), rotate( 0, 1), rotate( 0, 0), rotate( 1, 0)],
        }
    }

    pub fn cells_with_connections(&self) -> [(i32, i32, EnumSet<Direction>); 4] {
        use Direction::*;
        let rotate = |d: EnumSet<_>| match self.1 {
            RotationState::North => d,
            RotationState::East => d.iter().map(Direction::cw).collect(),
            RotationState::South => d.iter().map(Direction::flip).collect(),
            RotationState::West => d.iter().map(Direction::ccw).collect()
        };
        let cells = self.cells();
        [
            (cells[0].0, cells[0].1, rotate(match self.0 {
                Piece::I => enum_set!(Right),
                Piece::O => enum_set!(Right | Up),
                Piece::L => enum_set!(Right),
                Piece::J => enum_set!(Right | Up),
                Piece::T => enum_set!(Right),
                Piece::S => enum_set!(Right),
                Piece::Z => enum_set!(Right),
            })),
            (cells[1].0, cells[1].1, rotate(match self.0 {
                Piece::I => enum_set!(Left | Right),
                Piece::O => enum_set!(Left | Up),
                Piece::L => enum_set!(Left | Right),
                Piece::J => enum_set!(Left | Right),
                Piece::T => enum_set!(Left | Right | Up),
                Piece::S => enum_set!(Left | Up),
                Piece::Z => enum_set!(Left | Down),
            })),
            (cells[2].0, cells[2].1, rotate(match self.0 {
                Piece::I => enum_set!(Left | Right),
                Piece::O => enum_set!(Right | Down),
                Piece::L => enum_set!(Left | Up),
                Piece::J => enum_set!(Left),
                Piece::T => enum_set!(Left),
                Piece::S => enum_set!(Down | Right),
                Piece::Z => enum_set!(Up | Right),
            })),
            (cells[3].0, cells[3].1, rotate(match self.0 {
                Piece::I => enum_set!(Left),
                Piece::O => enum_set!(Left | Down),
                Piece::L => enum_set!(Down),
                Piece::J => enum_set!(Down),
                Piece::T => enum_set!(Down),
                Piece::S => enum_set!(Left),
                Piece::Z => enum_set!(Left),
            })),
        ]
    }
}

impl rand::distributions::Distribution<Piece> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Piece {
        match rng.gen_range(0, 7) {
            0 => Piece::I,
            1 => Piece::T,
            2 => Piece::O,
            3 => Piece::L,
            4 => Piece::J,
            5 => Piece::S,
            6 => Piece::Z,
            _ => unreachable!()
        }
    }
}

impl Piece {
    pub fn to_char(self) -> char {
        match self {
            Piece::I => 'I',
            Piece::T => 'T',
            Piece::O => 'O',
            Piece::L => 'L',
            Piece::J => 'J',
            Piece::S => 'S',
            Piece::Z => 'Z',
        }
    }

    pub fn color(self) -> CellColor {
        match self {
            Piece::I => CellColor::I,
            Piece::T => CellColor::T,
            Piece::O => CellColor::O,
            Piece::L => CellColor::L,
            Piece::J => CellColor::J,
            Piece::S => CellColor::S,
            Piece::Z => CellColor::Z,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum PieceMovement {
    Left,
    Right,
    Cw,
    Ccw,
    SonicDrop
}

impl PieceMovement {
    pub fn apply(self, piece: &mut FallingPiece, board: &Board) -> bool {
        match self {
            PieceMovement::Left => piece.shift(board, -1, 0),
            PieceMovement::Right => piece.shift(board, 1, 0),
            PieceMovement::Ccw => piece.ccw(board),
            PieceMovement::Cw => piece.cw(board),
            PieceMovement::SonicDrop => piece.sonic_drop(board)
        }
    }
}

#[derive(EnumSetType, Debug)]
pub enum Direction {
    Up, Down, Left, Right
}

impl Direction {
    fn cw(self) -> Direction {
        match self {
            Direction::Up => Direction::Right,
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
        }
    }

    fn ccw(self) -> Direction {
        match self {
            Direction::Up => Direction::Left,
            Direction::Right => Direction::Up,
            Direction::Down => Direction::Right,
            Direction::Left => Direction::Down,
        }
    }

    fn flip(self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
        }
    }
}