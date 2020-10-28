use libtetris::{ Board, FallingPiece, TspinStatus, PieceMovement };
use arrayvec::ArrayVec;
use std::collections::{ HashMap, HashSet };
use serde::{ Serialize, Deserialize };

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct InputList {
    pub movements: ArrayVec<[PieceMovement; 32]>,
    pub time: u32
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub inputs: InputList,
    pub location: FallingPiece
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Move {
    pub inputs: ArrayVec<[PieceMovement; 32]>,
    pub expected_location: FallingPiece,
    pub hold: bool
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MovementMode {
    ZeroG,
    ZeroGComplete,
    TwentyG,
    HardDropOnly
}

pub fn find_moves(
    board: &Board,
    mut spawned: FallingPiece,
    mode: MovementMode
) -> Vec<Placement> {
    let mut locks = HashMap::with_capacity(1024);
    let mut checked = HashSet::with_capacity(1024);
    let mut check_queue = vec![];
    let fast_mode;

    fast_mode = false;
    let mut movements = ArrayVec::new();
    if mode == MovementMode::TwentyG {
        spawned.sonic_drop(board);
        movements.push(PieceMovement::SonicDrop);
    }
    checked.insert(spawned);
    check_queue.push(Placement {
        inputs: InputList { movements, time: 0 },
        location: spawned
    });

    fn next(q: &mut Vec<Placement>) -> Option<Placement> {
        q.sort_by(|a, b|
            a.inputs.time.cmp(&b.inputs.time).then(
                a.inputs.movements.len().cmp(&b.inputs.movements.len())
            ).reverse()
        );
        q.pop()
    }

    while let Some(placement) = next(&mut check_queue) {
        let moves = placement.inputs;
        let position = placement.location;
        if !moves.movements.is_full() {
            attempt(
                board, &moves, position,
                &mut checked, &mut check_queue,
                mode, fast_mode,
                PieceMovement::Left, false
            );
            attempt(
                board, &moves, position,
                &mut checked, &mut check_queue,
                mode, fast_mode,
                PieceMovement::Right, false
            );

            attempt(
                board, &moves, position,
                &mut checked, &mut check_queue,
                mode, fast_mode,
                PieceMovement::Cw, false
            );

            attempt(
                board, &moves, position,
                &mut checked, &mut check_queue,
                mode, fast_mode,
                PieceMovement::Ccw, false
            );

            attempt(
                board, &moves, position,
                &mut checked, &mut check_queue,
                mode, fast_mode,
                PieceMovement::Flip, false
            );
            

            if mode == MovementMode::ZeroG {
                attempt(
                    board, &moves, position,
                    &mut checked, &mut check_queue,
                    mode, fast_mode,
                    PieceMovement::Left, true
                );

                attempt(
                    board, &moves, position,
                    &mut checked, &mut check_queue,
                    mode, fast_mode,
                    PieceMovement::Right, true
                );
            }

            attempt(
                board, &moves, position,
                &mut checked, &mut check_queue,
                mode, fast_mode,
                PieceMovement::SonicDrop, false
            );
        }

        let mut position = position;
        position.sonic_drop(board);
        lock_check(position, &mut locks, moves);
    }

    locks.into_iter().map(|(_, v)| v).collect()
}

fn lock_check(
    piece: FallingPiece,
    locks: &mut HashMap<([(i32, i32); 4], TspinStatus), Placement>,
    moves: InputList
) {
    let mut cells = piece.cells();
    if cells.iter().all(|&(_, y)| y >= 23) {
        return
    }
    cells.sort();

    // Since the first path to a location is always the shortest path to that location,
    // we know that if there is already an entry here this isn't a faster path, so only
    // insert placement if there isn't one there already.
    locks.entry((cells, piece.tspin)).or_insert(Placement {
        inputs: moves,
        location: piece,
    });
}

fn attempt(
    board: &Board,
    moves: &InputList,
    mut piece: FallingPiece,
    checked: &mut HashSet<FallingPiece>,
    check_queue: &mut Vec<Placement>,
    mode: MovementMode,
    fast_mode: bool,
    input: PieceMovement,
    repeat: bool
) -> FallingPiece {
    let orig_y = piece.y;
    if input.apply(&mut piece, board) {
        let mut moves = moves.clone();
        if input == PieceMovement::SonicDrop {
            // We don't actually know the soft drop speed, but 1 cell every 2 ticks is probably a
            // decent guess - that's what the battle library's default game configuration has, and
            // it's also pretty close to Puyo Puyo Tetris's versus mode.
            moves.time += 2 * (orig_y - piece.y) as u32;
        } else {
            moves.time += 1;
        }
        if let Some(&m) = moves.movements.last() {
            if m == input {
                // Delay from releasing button before pressing it again
                moves.time += 1;
            }
        }
        moves.movements.push(input);
        while repeat && !moves.movements.is_full() && input.apply(&mut piece, board) {
            // This is the DAS left/right case
            moves.movements.push(input);
            moves.time += 2;
        }
        if !fast_mode || piece.tspin != TspinStatus::None || !board.above_stack(&piece) {
            // 20G causes instant plummet, but we might actually be playing a high gravity mode
            // that we're approximating as 20G so we need to add a sonic drop movement to signal to
            // the input engine that we need the piece to hit the ground before continuing.
            let drop_input = mode == MovementMode::TwentyG && piece.sonic_drop(board);
            if checked.insert(piece) {
                if drop_input && !moves.movements.is_full() {
                    // We need the sonic drop input for the above reason, but if the move list is
                    // full this has to be the last move and the input engine should hard drop.
                    moves.movements.push(PieceMovement::SonicDrop);
                }
                if !(mode == MovementMode::HardDropOnly && input == PieceMovement::SonicDrop) {
                    check_queue.push(Placement { inputs: moves, location: piece });
                }
            }
        }
    }
    piece
}