use rand::prelude::*;
use enum_map::EnumMap;
use arrayvec::ArrayVec;
use odds::vec::VecExt;
use libtetris::{ Board, LockResult, Piece, FallingPiece };
use crate::moves::Placement;
use crate::evaluation::{ Evaluator, Evaluation, Eval, SearchOptions };
use crate::Options;

pub struct Tree {
    pub board: Board,
    pub raw_eval: Evaluation,
    pub evaluation: Eval,
    pub depth: usize,
    pub child_nodes: usize,
    kind: Option<TreeKind>
}

enum TreeKind {
    Known(Vec<Child>),
    Unknown(Speculation)
}

type Speculation = EnumMap<Piece, Option<Vec<Child>>>;

pub struct Child {
    pub hold: bool,
    pub mv: Placement,
    pub lock: LockResult,
    pub tree: Tree
}

impl Tree {
    pub fn starting_board(board: Board) -> Self {
        Tree {
            board,
            raw_eval: Default::default(),
            evaluation: Default::default(),
            depth: 0, child_nodes: 0, kind: None
        }
    }

    pub fn new(
        board: Board,
        lock: &LockResult,
        move_time: u32,
        piece: Piece,
        evaluator: &impl Evaluator
    ) -> Self {
        let raw_eval = evaluator.evaluate(lock, &board, move_time, piece);
        Tree {
            raw_eval, board,
            evaluation: raw_eval.into(),
            depth: 0,
            child_nodes: 0,
            kind: None
        }
    }

    pub fn into_best_child(mut self) -> Result<Child, Tree> {
        match self.kind {
            None => Err(self),
            Some(tk) => {
                match tk.into_best_child() {
                    Ok(c) => Ok(c),
                    Err(tk) => {
                        self.kind = Some(tk);
                        Err(self)
                    }
                }
            }
        }
    }

    pub fn get_plan(&self, into: &mut Vec<(Placement, LockResult)>) {
        if let Some(ref tk) = self.kind {
            tk.get_plan(into);
        }
    }

    pub fn get_moves_and_evaluations(&self) -> Vec<(FallingPiece, Eval)> {
        if let Some(ref tk) = self.kind {
            tk.get_moves_and_evaluations()
        } else {
            vec![]
        }
    }

    /// Returns is_death
    pub fn add_next_piece(&mut self, piece: Piece, options: SearchOptions) -> bool {
        self.board.add_next_piece(piece);
        if let Some(ref mut k) = self.kind {
            if k.add_next_piece(piece, options) {
                true
            } else {
                self.evaluation = k.evaluation() * options.gamma.0 / options.gamma.1
                    + self.raw_eval;
                false
            }
        } else {
            false
        }
    }

    /// Does an iteration of MCTS. Returns true if only death is possible from this position.
    pub fn extend(
        &mut self, opts: Options, evaluator: &impl Evaluator
    ) -> bool {
        self.expand(opts, evaluator).is_death
    }

    fn expand(
        &mut self, opts: Options, evaluator: &impl Evaluator
    ) -> ExpandResult {
        match self.kind {
            // TODO: refactor the unexpanded case into TreeKind
            Some(ref mut tk) => {
                let er = tk.expand(opts, evaluator);
                if !er.is_death {
                    // Update this node's information
                    let opts = evaluator.search_options();
                    self.evaluation = tk.evaluation() * opts.gamma.0 / opts.gamma.1
                        + self.raw_eval;
                    self.depth = self.depth.max(er.depth);
                    self.child_nodes += er.new_nodes;
                }
                er
            }
            None => {
                if self.board.get_next_piece().is_ok() {
                    if opts.use_hold && self.board.hold_piece().is_none() &&
                            self.board.get_next_next_piece().is_none() {
                        // Speculate - next piece is known, but hold piece isn't
                        self.speculate(opts, evaluator)
                    } else {
                        // Both next piece and hold piece are known
                        let children = new_children(
                            self.board.clone(), opts, evaluator
                        );

                        if children.is_empty() {
                            ExpandResult {
                                is_death: true,
                                depth: 0,
                                new_nodes: 0
                            }
                        } else {
                            self.depth = 1;
                            self.child_nodes = children.len();
                            let tk = TreeKind::Known(children);
                            let opts = evaluator.search_options();
                            self.evaluation = tk.evaluation() * opts.gamma.0 / opts.gamma.1
                                + self.raw_eval;
                            self.kind = Some(tk);
                            ExpandResult {
                                is_death: false,
                                depth: 1,
                                new_nodes: self.child_nodes
                            }
                        }
                    }
                } else {
                    // Speculate - hold should be known, but next piece isn't
                    assert!(
                        opts.use_hold && self.board.hold_piece().is_some(),
                        "Neither hold piece or next piece are known - what the heck happened?\n\
                         get_next_piece: {:?}", self.board.get_next_piece()
                    );
                    self.speculate(opts, evaluator)
                }
            }
        }
    }

    fn speculate(
        &mut self,
        opts: Options,
        evaluator: &impl Evaluator
    ) -> ExpandResult {
        if !opts.speculate {
            return ExpandResult {
                is_death: false,
                depth: 0,
                new_nodes: 0
            }
        }
        let possibilities = match self.board.get_next_piece() {
            Ok(_) => {
                let mut b = self.board.clone();
                b.advance_queue();
                b.get_next_piece().unwrap_err()
            }
            Err(possibilities) => possibilities
        };
        let mut speculation = EnumMap::new();
        for piece in possibilities.iter() {
            let mut board = self.board.clone();
            board.add_next_piece(piece);
            let children = new_children(
                board, opts, evaluator
            );
            self.child_nodes += children.len();
            speculation[piece] = Some(children);
        }

        if self.child_nodes == 0 {
            ExpandResult {
                is_death: true,
                depth: 0,
                new_nodes: 0
            }
        } else {
            let tk = TreeKind::Unknown(speculation);
            let opts = evaluator.search_options();
            self.evaluation = tk.evaluation() * opts.gamma.0 / opts.gamma.1
                + self.raw_eval;
            self.kind = Some(tk);
            self.depth = 1;
            ExpandResult {
                is_death: false,
                depth: 1,
                new_nodes: self.child_nodes
            }
        }
    }
}

/// Expect: If there is no hold piece, there are at least 2 pieces in the queue.
/// Otherwise there is at least 1 piece in the queue.
fn new_children(
    mut board: Board,
    opts: Options,
    evaluator: &impl Evaluator
) -> Vec<Child> {
    let mut children = vec![];
    let next = board.advance_queue().unwrap();
    let spawned = match FallingPiece::spawn(next, &board) {
        Some(s) => s,
        None => return children
    };

    // Placements for next piece
    for mv in crate::moves::find_moves(&board, spawned, opts.mode) {
        let mut board = board.clone();
        let lock = board.lock_piece(mv.location);
        if !lock.locked_out {
            children.push(Child {
                tree: Tree::new(board, &lock, mv.inputs.time, next, evaluator),
                hold: false,
                mv, lock
            })
        }
    }

    if opts.use_hold {
        let mut board = board.clone();
        let hold = board.hold(next).unwrap_or_else(|| board.advance_queue().unwrap());
        if hold != next {
            if let Some(spawned) = FallingPiece::spawn(hold, &board) {
                // Placements for hold piece
                for mv in crate::moves::find_moves(&board, spawned, opts.mode) {
                    let mut board = board.clone();
                    let lock = board.lock_piece(mv.location);
                    if !lock.locked_out {
                        children.push(Child {
                            tree: Tree::new(board, &lock, mv.inputs.time, hold, evaluator),
                            hold: true,
                            mv, lock
                        })
                    }
                }
            }
        }
    }

    children
}

struct ExpandResult {
    depth: usize,
    new_nodes: usize,
    is_death: bool
}

impl TreeKind {
    fn into_best_child(self) -> Result<Child, TreeKind> {
        match self {
            TreeKind::Known(children) => if children.is_empty() {
                Err(TreeKind::Known(children))
            } else {
                Ok(children.into_iter().next().unwrap())
            },
            TreeKind::Unknown(_) => Err(self),
        }
    }

    fn get_plan(&self, into: &mut Vec<(Placement, LockResult)>) {
        match self {
            TreeKind::Known(children) => if let Some(mv) = children.first() {
                into.push((mv.mv.clone(), mv.lock.clone()));
                mv.tree.get_plan(into);
            }
            _ => {}
        }
    }

    fn get_moves_and_evaluations(&self) -> Vec<(FallingPiece, Eval)> {
        match self {
            TreeKind::Known(children) => children.iter()
                .map(|c| (c.mv.location, c.tree.evaluation))
                .collect(),
            _ => vec![]
        }
    }

    fn evaluation(&self) -> Eval {
        match self {
            TreeKind::Known(children) => best_eval(children).unwrap(),
            TreeKind::Unknown(speculation) => {
                let mut sum = Eval { aggressive: 0, defensive: 0 };
                let mut n = 0;
                let mut deaths = 0;
                for children in speculation.iter().filter_map(|(_, c)| c.as_ref()) {
                    match best_eval(children) {
                        Some(v) => {
                            n += 1;
                            sum.aggressive += v.aggressive;
                            sum.defensive += v.defensive;
                        }
                        None => deaths += 1,
                    }
                }
                let avg_value = sum / n;
                sum.aggressive += (avg_value.aggressive - 1000) * deaths;
                sum.defensive += (avg_value.defensive - 1000) * deaths;
                sum / (n + deaths)
            }
        }
    }

    /// Returns is_death
    fn add_next_piece(&mut self, piece: Piece, opts: SearchOptions) -> bool {
        match self {
            TreeKind::Known(children) => {
                children.retain_mut(|child|
                    !child.tree.add_next_piece(piece, opts)
                );
                children.is_empty()
            }
            TreeKind::Unknown(speculation) => {
                let mut now_known = vec![];
                std::mem::swap(speculation[piece].as_mut().unwrap(), &mut now_known);
                let is_death = now_known.is_empty();
                *self = TreeKind::Known(now_known);
                is_death
            }
        }
    }

    fn expand(
        &mut self,
        opts: Options,
        evaluator: &impl Evaluator
    ) -> ExpandResult {
        let to_expand = match self {
            TreeKind::Known(children) => children,
            TreeKind::Unknown(speculation) => {
                let mut pieces = ArrayVec::<[Piece; 7]>::new();
                for (piece, children) in speculation.iter() {
                    if let Some(children) = children {
                        if !children.is_empty() {
                            pieces.push(piece);
                        }
                    }
                }
                speculation[*pieces.choose(&mut thread_rng()).unwrap()].as_mut().unwrap()
            }
        };
        if to_expand.is_empty() {
            return ExpandResult {
                depth: 0,
                new_nodes: 0,
                is_death: true
            }
        }

        to_expand.sort_by_key(|c| {
            let h = c.tree.board.column_heights().iter().sum::<i32>() / 10;
            -c.tree.evaluation.value(h, evaluator.search_options())
        });

        let min = {
            let t = &to_expand.last().unwrap().tree;
            let h = t.board.column_heights().iter().sum::<i32>() / 10;
            t.evaluation.value(h, evaluator.search_options())
        };

        let weights = to_expand.iter()
            .enumerate()
            .map(|(i, c)| {
                let h = c.tree.board.column_heights().iter().sum::<i32>() / 10;
                let e = (c.tree.evaluation.value(h, evaluator.search_options()) - min) as i64;
                e * e / (i + 1) as i64 + 1
            });
        let sampler = rand::distributions::WeightedIndex::new(weights).unwrap();
        let index = thread_rng().sample(sampler);

        let result = to_expand[index].tree.expand(opts, evaluator);
        if result.is_death {
            to_expand.remove(index);
            match self {
                TreeKind::Known(children) => if children.is_empty() {
                    return ExpandResult {
                        is_death: true,
                        depth: result.depth + 1,
                        ..result
                    }
                }
                TreeKind::Unknown(speculation) => if speculation.iter()
                        .all(|(_, c)| c.as_ref().map(Vec::is_empty).unwrap_or(true)) {
                    return ExpandResult {
                        is_death: true,
                        depth: result.depth + 1,
                        ..result
                    }
                }
            }
            ExpandResult {
                is_death: false,
                depth: result.depth + 1,
                ..result
            }
        } else {
            ExpandResult {
                depth: result.depth + 1,
                ..result
            }
        }
    }
}

fn best_eval(children: &[Child]) -> Option<Eval> {
    if let Some(first) = children.first() {
        Some(children[1..].iter().fold(
            first.tree.evaluation,
            |acc, c| {
                Eval {
                    aggressive: acc.aggressive.max(c.tree.evaluation.aggressive),
                    defensive: acc.defensive.max(c.tree.evaluation.defensive),
                }
            }
        ))
    } else {
        None
    }
}