use std::collections::HashSet;
use game_sim::input::Input;

/// Player 1: WASD + Z (light) + X (heavy) + C (special) + V (block)
pub fn read_p1_input(keys: &HashSet<String>) -> Input {
    let mut bits: u8 = 0;
    if keys.contains("a") || keys.contains("A") {
        bits |= 1 << 0; // left
    }
    if keys.contains("d") || keys.contains("D") {
        bits |= 1 << 1; // right
    }
    if keys.contains("w") || keys.contains("W") {
        bits |= 1 << 2; // up/jump
    }
    if keys.contains("s") || keys.contains("S") {
        bits |= 1 << 3; // down/crouch
    }
    if keys.contains("z") || keys.contains("Z") {
        bits |= 1 << 4; // light attack
    }
    if keys.contains("x") || keys.contains("X") {
        bits |= 1 << 5; // heavy attack
    }
    if keys.contains("c") || keys.contains("C") {
        bits |= 1 << 6; // special
    }
    if keys.contains("v") || keys.contains("V") {
        bits |= 1 << 7; // block
    }
    Input(bits)
}

/// Player 2: Arrow keys + J (light) + K (heavy) + L (special) + ; (block)
pub fn read_p2_input(keys: &HashSet<String>) -> Input {
    let mut bits: u8 = 0;
    if keys.contains("ArrowLeft") {
        bits |= 1 << 0;
    }
    if keys.contains("ArrowRight") {
        bits |= 1 << 1;
    }
    if keys.contains("ArrowUp") {
        bits |= 1 << 2;
    }
    if keys.contains("ArrowDown") {
        bits |= 1 << 3;
    }
    if keys.contains("j") || keys.contains("J") {
        bits |= 1 << 4;
    }
    if keys.contains("k") || keys.contains("K") {
        bits |= 1 << 5;
    }
    if keys.contains("l") || keys.contains("L") {
        bits |= 1 << 6;
    }
    if keys.contains(";") {
        bits |= 1 << 7;
    }
    Input(bits)
}
