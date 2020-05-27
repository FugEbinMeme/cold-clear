use arrayvec::ArrayVec;
use serde::{ Serialize, Deserialize };

use crate::piece::TspinStatus;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub struct LockResult {
    pub placement_kind: PlacementKind,
    pub b2b: bool,
    pub perfect_clear: bool,
    pub combo: Option<u32>,
    pub garbage_sent: u32,
    pub cleared_lines: ArrayVec<[i32; 4]>
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlacementKind {
    None,
    Clear1,
    Clear2,
    Clear3,
    Clear4,
    Tspin,
    Tspin1,
    Tspin2,
    Tspin3,
    Tspin4
}

impl PlacementKind {
    /// The amount of garbage this clear kind normally sends.
    pub fn garbage(self) -> u32 {
        use PlacementKind::*;
        match self {
            None | Tspin | Clear1 => 0,
            Clear2 => 1,
            Clear3 | Tspin1 => 2,
            Clear4 | Tspin2 => 4,
            Tspin3 => 6,
            Tspin4 => 8
        }
    }

    /// Whether or not this placement does back-to-backs.
    pub fn is_hard(self) -> bool {
        use PlacementKind::*;
        match self {
            Clear4 |
            Tspin | Tspin1 | Tspin2 | Tspin3 | Tspin4 => true,
            _ => false
        }
    }

    /// Whether or not this placement did a line clear.
    pub fn is_clear(self) -> bool {
        match self {
            PlacementKind::None | PlacementKind::Tspin => false,
            _ => true
        }
    }

    pub(crate) fn get(cleared: usize, tspin: TspinStatus) -> Self {
        match (cleared, tspin) {
            (0, TspinStatus::None) => PlacementKind::None,
            (0, _)                 => PlacementKind::Tspin,
            (1, TspinStatus::None) => PlacementKind::Clear1,
            (1, _)                 => PlacementKind::Tspin1,
            (2, TspinStatus::None) => PlacementKind::Clear2,
            (2, _)                 => PlacementKind::Tspin2,
            (3, TspinStatus::None) => PlacementKind::Clear3,
            (3, _)                 => PlacementKind::Tspin3,
            (4, TspinStatus::None) => PlacementKind::Clear4,
            (4, _)                 => PlacementKind::Tspin4,
            _ => unreachable!()
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PlacementKind::None       => "",
            PlacementKind::Clear1     => "Single",
            PlacementKind::Clear2     => "Double",
            PlacementKind::Clear3     => "Triple",
            PlacementKind::Clear4     => "Quad",
            PlacementKind::Tspin      => "T-Spin",
            PlacementKind::Tspin1     => "T-Spin Single",
            PlacementKind::Tspin2     => "T-Spin Double",
            PlacementKind::Tspin3     => "T-Spin Triple",
            PlacementKind::Tspin4     => "T-Spin Quad",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            PlacementKind::None       => "...",
            PlacementKind::Clear1     => "S",
            PlacementKind::Clear2     => "D",
            PlacementKind::Clear3     => "T",
            PlacementKind::Clear4     => "Q",
            PlacementKind::Tspin      => "TS",
            PlacementKind::Tspin1     => "TSS",
            PlacementKind::Tspin2     => "TSD",
            PlacementKind::Tspin3     => "TST",
            PlacementKind::Tspin4     => "TSQ",
        }
    }
}

impl Default for PlacementKind {
    fn default() -> Self {
        PlacementKind::None
    }
}

pub const COMBO_GARBAGE: [u32; 12] = [
    0, 0,       // 0, 1 combo
    1, 1,       // 2, 3 combo
    2, 2,       // 4, 5 combo
    3, 3,       // 6, 7 combo
    4, 4, 4,    // 8, 9, 10 combo
    5           // 11+ combo
];

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, Hash, Serialize, Deserialize)]
pub struct Statistics {
    pub pieces: u64,
    pub lines: u64,
    pub attack: u64,

    pub singles: u64,
    pub doubles: u64,
    pub triples: u64,
    pub quads: u64,
    pub tspin_zeros: u64,
    pub tspin_singles: u64,
    pub tspin_doubles: u64,
    pub tspin_triples: u64,
    pub tspin_quads: u64,
    pub perfect_clears: u64,
    pub max_combo: u64
}

impl Statistics {
    pub fn update(&mut self, l: &LockResult) {
        self.attack += l.garbage_sent as u64;
        self.lines += l.cleared_lines.len() as u64;
        self.pieces += 1;

        if l.perfect_clear {
            self.perfect_clears += 1;
        }
        if let Some(combo) = l.combo {
            if combo as u64 > self.max_combo {
                self.max_combo = combo as u64;
            }
        }

        match l.placement_kind {
            PlacementKind::None => {},
            PlacementKind::Clear1 => self.singles += 1,
            PlacementKind::Clear2 => self.doubles += 1,
            PlacementKind::Clear3 => self.triples += 1,
            PlacementKind::Clear4 => self.quads += 1,
            PlacementKind::Tspin => self.tspin_zeros += 1,
            PlacementKind::Tspin1 => self.tspin_singles += 1,
            PlacementKind::Tspin2 => self.tspin_doubles += 1,
            PlacementKind::Tspin3 => self.tspin_triples += 1,
            PlacementKind::Tspin4 => self.tspin_quads += 1,
        }
    }
}