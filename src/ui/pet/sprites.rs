//! Pixel-art sprite data for the Kyoku pet.
//!
//! Every grid is a rectangular matrix of single-character palette tokens. Each
//! row is one pixel row, and the renderer folds two pixel rows into one
//! terminal row using ANSI half-blocks, so a 12-row grid draws in 6 terminal
//! rows. Grids come in three sizes so the cat fills whatever pane the layout
//! hands it: a 16x16 set for a tall deck, a 12x12 set for a medium one, and a
//! compact 10x8 set for short panes.
//!
//! Palette tokens:
//!   `.` transparent   `F` fur          `f` fur (dim / whisker)
//!   `P` inner ear     `B` blush        `N` nose
//!   `M` muzzle        `H` headband     `C` headphone cup
//!   `c` headphone glow `E` eye dark    `W` eye shine
//!   `A` collar

/// Mid-size 12x12 dancing frames, drawn in 6 terminal rows.
pub const MEDIUM_PLAYING: [[&str; 12]; 4] = [
    // Frame 0: groove left, tail flicked right
    [
        "F........F..",
        "FP......PF..",
        "FPP.HH.PPF.f",
        "FFFHHHHFFF.f",
        "CFFFFFFFFC.f",
        "CFEWFFWEFC..",
        "cFBFFNNFBFc.",
        "f.FFMMFF.f..",
        "..AAAAAA...f",
        ".FFFFFFFF..f",
        ".FFFFFFFFFf.",
        ".PP.FF.PP...",
    ],
    // Frame 1: upright bounce, ears at full height
    [
        ".F........F.",
        ".FP......PF.",
        "FFPP.HH.PPFF",
        "FFFFHHHHFFFF",
        "CFFFFFFFFFFC",
        "CFEWFFFFWEFC",
        "cFBFFNNFFBFc",
        "f.FFFMMFFF.f",
        "...AAAAAA..f",
        "..FFFFFFFF.f",
        "..FFFFFFFFFf",
        "..PP.FF.PP..",
    ],
    // Frame 2: groove right, tail flicked left
    [
        "..F........F",
        "..FP......PF",
        "f.FPP.HH.PPF",
        "f.FFFHHHHFFF",
        "f.CFFFFFFFFC",
        "..CFEWFFWEFC",
        ".cFBFFNNFBFc",
        "..f.FFMMFF.f",
        "f...AAAAAA..",
        "f..FFFFFFFF.",
        ".fFFFFFFFFF.",
        "...PP.FF.PP.",
    ],
    // Frame 3: bass drop, eyes squeezed shut, wide stance
    [
        "F..........F",
        ".FP......PF.",
        "FFPP.HH.PPFF",
        "FFFFHHHHFFFF",
        "cFFFFFFFFFFc",
        "cFffFFFFffFc",
        "cFBFFNNFFBFc",
        "f.FFFMMFFF.f",
        "..AAAAAAAA..",
        ".FFFFFFFFFF.",
        "FFFFFFFFFFFf",
        ".PP.FFFF.PPf",
    ],
];

/// Mid-size 12x12 sleeping frames, drawn in 6 terminal rows.
pub const MEDIUM_SLEEPING: [[&str; 12]; 2] = [
    // Frame 0: inhale
    [
        "............",
        ".f........f.",
        ".fP......Pf.",
        "ffPP.HH.PPff",
        "ffffffffffff",
        "cfEEffffEEfc",
        "cfBffNNffBfc",
        "f.fffMMfff.f",
        "..AAAAAAAA..",
        ".ffffffffff.",
        "ffffffffffff",
        ".PP.ffff.PPf",
    ],
    // Frame 1: exhale, body settles a row lower
    [
        "............",
        "............",
        ".f........f.",
        ".fPP.HH.PPf.",
        "ffffffffffff",
        "cfEEffffEEfc",
        "cfBffNNffBfc",
        "f.fffMMfff.f",
        "..AAAAAAAA..",
        "ffffffffffff",
        "ffffffffffff",
        ".PP.ffff.PP.",
    ],
];

/// Compact 10x8 dancing frames, drawn in 4 terminal rows.
pub const SMALL_PLAYING: [[&str; 8]; 4] = [
    // Frame 0: groove left
    [
        "F......F..",
        "FP....PF..",
        "FFFHHFFFC.",
        "FEWFFWEFC.",
        "FBFNNFBFc.",
        "FAAAAAAF.f",
        "FFFFFFFF.f",
        "PP....PPf.",
    ],
    // Frame 1: upright bounce
    [
        ".F......F.",
        ".FP....PF.",
        "CFFFHHFFFC",
        "CFEWFFWEFC",
        "cFBFNNFBFc",
        ".FAAAAAAF.",
        ".FFFFFFFFf",
        ".PP....PPf",
    ],
    // Frame 2: groove right
    [
        "..F......F",
        "..FP....PF",
        ".CFFFHHFFF",
        ".CFEWFFWEF",
        ".cFBFNNFBF",
        "f.FAAAAAAF",
        "f.FFFFFFFF",
        ".fPP....PP",
    ],
    // Frame 3: bass drop, eyes shut
    [
        ".F......F.",
        ".FP....PF.",
        "cFFFHHFFFc",
        "cFffFFffFc",
        "cFBFNNFBFc",
        ".AAAAAAAA.",
        "FFFFFFFFFF",
        "PP.FFFF.PP",
    ],
];

/// Compact 10x8 sleeping frames, drawn in 4 terminal rows.
pub const SMALL_SLEEPING: [[&str; 8]; 2] = [
    // Frame 0: inhale
    [
        "..........",
        ".f......f.",
        ".fP....Pf.",
        "cffHHHHffc",
        "cfEEffEEfc",
        "cfBfNNfBfc",
        ".ffffffff.",
        "fPP....PPf",
    ],
    // Frame 1: exhale
    [
        "..........",
        "..........",
        ".f......f.",
        ".fP.HH.Pf.",
        "cfEEffEEfc",
        "cfBfNNfBfc",
        "ffffffffff",
        ".PP....PP.",
    ],
];

/// Roomy 16x16 dancing frames, drawn in 8 terminal rows. This is the most
/// detailed cat: three-pixel triangular ears, two-pixel eyes with a shine,
/// whiskers, a muzzle, and a tail sweeping up one hip.
pub const LARGE_PLAYING: [[&str; 16]; 4] = [
    // Frame 0: groove left, tail swings right
    [
        "..F........F....",
        "..FP......PF....",
        ".FFP......PFF...",
        "FFPPP.HH.PPPFF..",
        "FFFFFHHHHFFFFF..",
        "FFFFFFFFFFFFFFC.",
        "FFWEEFFFFEEWFFC.",
        "FFEEEFFFFEEEFFc.",
        "FBBFFFNNFFFBBFc.",
        ".FFFFMMMMFFFF.f.",
        "..FFFFFFFFFF..f.",
        "...AAAAAAAA....F",
        "..FFFFFFFFFF..FF",
        ".FFFFFFFFFFFF.FF",
        ".FFFFFFFFFFFFFF.",
        ".PPFFFFFFFFPP...",
    ],
    // Frame 1: upright bounce, ears at full height
    [
        "...F........F...",
        "...FP......PF...",
        "..FFP......PFF..",
        ".FFPPP.HH.PPPFF.",
        ".FFFFFHHHHFFFFF.",
        "CFFFFFFFFFFFFFFC",
        "CFFWEEFFFFEEWFFC",
        "cFFEEEFFFFEEEFFc",
        "cFBBFFFNNFFFBBFc",
        "f.FFFFMMMMFFFF.f",
        "f..FFFFFFFFFF..f",
        "....AAAAAAAA....",
        "...FFFFFFFFFF..F",
        "..FFFFFFFFFFFF.F",
        "..FFFFFFFFFFFFFF",
        "..PPFFFFFFFFPP..",
    ],
    // Frame 2: groove right, tail swings left
    [
        "....F........F..",
        "....FP......PF..",
        "...FFP......PFF.",
        "..FFPPP.HH.PPPFF",
        "..FFFFFHHHHFFFFF",
        ".CFFFFFFFFFFFFFF",
        ".CFFWEEFFFFEEWFF",
        ".cFFEEEFFFFEEEFF",
        ".cFBBFFFNNFFFBBF",
        ".f.FFFFMMMMFFFF.",
        ".f..FFFFFFFFFF..",
        "F....AAAAAAAA...",
        "FF..FFFFFFFFFF..",
        "FF.FFFFFFFFFFFF.",
        ".FFFFFFFFFFFFFF.",
        "...PPFFFFFFFFPP.",
    ],
    // Frame 3: bass drop, ears flared, eyes squeezed shut, wide stance
    [
        "...F........F...",
        "..FFP......PFF..",
        ".FFPPP....PPPFF.",
        ".FFPPP.HH.PPPFF.",
        ".FFFFFHHHHFFFFF.",
        "cFFFFFFFFFFFFFFc",
        "cFFFFFFFFFFFFFFc",
        "cFFEEEFFFFEEEFFc",
        "cFBBFFFNNFFFBBFc",
        "f.FFFFMMMMFFFF.f",
        "f..FFFFFFFFFF..f",
        "...AAAAAAAAAA...",
        "..FFFFFFFFFFFF..",
        ".FFFFFFFFFFFFFFF",
        "FFFFFFFFFFFFFFFF",
        ".PP.FFFFFFFF.PP.",
    ],
];

/// Roomy 16x16 sleeping frames, drawn in 8 terminal rows.
pub const LARGE_SLEEPING: [[&str; 16]; 2] = [
    // Frame 0: inhale
    [
        "................",
        "...f........f...",
        "...fP......Pf...",
        "..ffP......Pff..",
        ".ffPPP.HH.PPPff.",
        ".fffffHHHHfffff.",
        "CffffffffffffffC",
        "CffEEEffffEEEffC",
        "cfBBfffNNfffBBfc",
        "f.ffffMMMMffff.f",
        "f..ffffffffff..f",
        "....AAAAAAAA....",
        "..ffffffffffff..",
        ".ffffffffffffff.",
        "ffffffffffffffff",
        ".PP.ffffffff.PP.",
    ],
    // Frame 1: exhale, the loaf settles a row lower and spreads
    [
        "................",
        "................",
        "...f........f...",
        "...fP......Pf...",
        "..ffPP....PPff..",
        ".ffPPP.HH.PPPff.",
        ".fffffHHHHfffff.",
        "CffffffffffffffC",
        "CffEEEffffEEEffC",
        "cfBBfffNNfffBBfc",
        "f.ffffMMMMffff.f",
        "f..ffffffffff..f",
        "...AAAAAAAAAA...",
        ".ffffffffffffff.",
        "ffffffffffffffff",
        ".PP.ffffffff.PP.",
    ],
];
