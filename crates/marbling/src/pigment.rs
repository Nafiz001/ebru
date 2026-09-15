/// An ink. Marblers grind earth and mineral pigments into water with a drop
/// of ox gall, which makes the ink spread on the bath instead of sinking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pigment {
    pub name: &'static str,
    pub rgb: [u8; 3],
}

impl Pigment {
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.rgb[0], self.rgb[1], self.rgb[2])
    }
}

/// The inks on the rail, by index. The index is what travels over the wire.
pub const PIGMENTS: [Pigment; 10] = [
    Pigment { name: "Lapis", rgb: [0x2b, 0x4f, 0x96] },
    Pigment { name: "Indigo", rgb: [0x1e, 0x2a, 0x55] },
    Pigment { name: "Madder", rgb: [0xb4, 0x3b, 0x2d] },
    Pigment { name: "Oxblood", rgb: [0x6a, 0x1d, 0x24] },
    Pigment { name: "Ochre", rgb: [0xcc, 0x92, 0x2f] },
    Pigment { name: "Verdigris", rgb: [0x3d, 0x8c, 0x78] },
    Pigment { name: "Rose", rgb: [0xd9, 0x7b, 0x82] },
    Pigment { name: "Umber", rgb: [0x6b, 0x4a, 0x31] },
    Pigment { name: "Oak gall", rgb: [0x1c, 0x1a, 0x1d] },
    Pigment { name: "Chalk", rgb: [0xf2, 0xef, 0xe6] },
];

/// Pigments a tray can be started on. A first layer of ink spread over the
/// whole bath, the ground, is traditional; the pattern is built on top.
pub const GROUNDS: [u8; 6] = [9, 1, 3, 4, 5, 7];

pub fn pigment(index: u8) -> Option<&'static Pigment> {
    PIGMENTS.get(index as usize)
}
