mod board;
mod piece;
mod lock_data;

pub use board::*;
pub use piece::*;
pub use lock_data::*;

#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq)]
pub struct Controller {
    pub left: bool,
    pub right: bool,
    pub rotate_right: bool,
    pub rotate_left: bool,
    pub rotate_180: bool,
    pub meme_flip: bool,
    pub soft_drop: bool,
    pub hard_drop: bool,
    pub hold: bool
}

impl serde::Serialize for Controller {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u16(
            (self.left as u16)         << 0 |
            (self.right as u16)        << 1 |
            (self.rotate_left as u16)  << 2 |
            (self.rotate_right as u16) << 3 |
            (self.rotate_180 as u16)   << 4 |
            (self.hold as u16)         << 5 |
            (self.soft_drop as u16)    << 6 |
            (self.hard_drop as u16)    << 7 |
            (self.meme_flip as u16)    << 8 
        )
    }
}

impl<'de> serde::Deserialize<'de> for Controller {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ControllerDeserializer;
        impl serde::de::Visitor<'_> for ControllerDeserializer {
            type Value = Controller;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a byte-sized bit vector")
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Controller, E> {
                Ok(Controller {
                    left:         (v >> 0) & 1 != 0,
                    right:        (v >> 1) & 1 != 0,
                    rotate_left:  (v >> 2) & 1 != 0,
                    rotate_right: (v >> 3) & 1 != 0,
                    rotate_180:   (v >> 4) & 1 != 0,
                    hold:         (v >> 5) & 1 != 0,
                    soft_drop:    (v >> 6) & 1 != 0,
                    hard_drop:    (v >> 7) & 1 != 0,
                    meme_flip:    (v >> 8) & 1 != 0,
                })
            }
        }
        deserializer.deserialize_u16(ControllerDeserializer)
    }
}