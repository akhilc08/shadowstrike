use game_sim::constants::{MAX_ENERGY, MAX_HEALTH, TICKS_PER_SECOND};
use game_sim::player::{Element, PlayerAction, PlayerState};
use game_sim::{GameState, Projectile, MAX_PROJECTILES};
use web_sys::CanvasRenderingContext2d;

use crate::animation::{compute_skeleton, get_animation, AnimationState, Joint, JointId};

const CANVAS_W: f64 = 1200.0;
const CANVAS_H: f64 = 600.0;
const GROUND_Y: f64 = 500.0;

/// Element accent colors.
fn element_accent(element: Element) -> &'static str {
    match element {
        Element::Fire => "#ff6600",
        Element::Lightning => "#88ccff",
        Element::DarkMagic => "#9900ff",
        Element::Ice => "#66eeff",
    }
}

fn element_glow(element: Element) -> &'static str {
    match element {
        Element::Fire => "rgba(255,102,0,0.35)",
        Element::Lightning => "rgba(100,180,255,0.35)",
        Element::DarkMagic => "rgba(153,0,255,0.35)",
        Element::Ice => "rgba(102,238,255,0.35)",
    }
}


pub fn render_frame(
    ctx: &CanvasRenderingContext2d,
    state: &GameState,
    anim_states: &[AnimationState; 2],
    hit_flash: &[i32; 2],
) {
    // Clear
    ctx.set_fill_style_str("#0a0a12");
    ctx.fill_rect(0.0, 0.0, CANVAS_W, CANVAS_H);

    draw_background(ctx);

    // Draw each player
    for (i, anim_state) in anim_states.iter().enumerate() {
        let p = &state.players[i];
        let facing = p.facing as f32;
        let anim = get_animation(anim_state.anim_id);
        let skeleton = compute_skeleton(p.x.to_f32(), p.y.to_f32(), facing, &anim, anim_state.frame);
        let flash = hit_flash[i] > 0;
        draw_character(ctx, p, &skeleton, facing, flash, i);
    }

    // UI
    draw_health_bar(ctx, &state.players[0], 0);
    draw_health_bar(ctx, &state.players[1], 1);
    draw_energy_bar(ctx, state.players[0].energy, 30.0, 58.0, false);
    draw_energy_bar(ctx, state.players[1].energy, CANVAS_W as f32 - 30.0, 58.0, true);
    draw_timer(ctx, state.round_timer);
    draw_round_indicators(ctx, &state.round_scores, &[state.players[0].element, state.players[1].element]);
    draw_combo_counter(ctx, state);
}

fn draw_background(ctx: &CanvasRenderingContext2d) {
    // Sky gradient
    let gradient = ctx.create_linear_gradient(0.0, 0.0, 0.0, GROUND_Y);
    let _ = gradient.add_color_stop(0.0, "#06061a");
    let _ = gradient.add_color_stop(0.5, "#0a0a24");
    let _ = gradient.add_color_stop(1.0, "#141430");
    ctx.set_fill_style_canvas_gradient(&gradient);
    ctx.fill_rect(0.0, 0.0, CANVAS_W, GROUND_Y);

    // Ground with subtle gradient
    let ground_gradient = ctx.create_linear_gradient(0.0, GROUND_Y, 0.0, CANVAS_H);
    let _ = ground_gradient.add_color_stop(0.0, "#1a1a2e");
    let _ = ground_gradient.add_color_stop(1.0, "#0f0f1a");
    ctx.set_fill_style_canvas_gradient(&ground_gradient);
    ctx.fill_rect(0.0, GROUND_Y, CANVAS_W, CANVAS_H - GROUND_Y);

    // Perspective grid lines on the ground
    ctx.set_stroke_style_str("rgba(60,60,100,0.15)");
    ctx.set_line_width(1.0);
    let vanish_x = CANVAS_W / 2.0;
    let vanish_y = GROUND_Y - 80.0;
    for i in 0..12 {
        let x = (i as f64) * 110.0 - 10.0;
        ctx.begin_path();
        ctx.move_to(x, CANVAS_H);
        ctx.line_to(vanish_x, vanish_y);
        ctx.stroke();
    }
    // Horizontal depth lines
    for i in 1..5 {
        let t = i as f64 / 5.0;
        let y = GROUND_Y + (CANVAS_H - GROUND_Y) * t;
        ctx.set_stroke_style_str(&format!("rgba(60,60,100,{:.2})", 0.08 + t * 0.1));
        ctx.begin_path();
        ctx.move_to(0.0, y);
        ctx.line_to(CANVAS_W, y);
        ctx.stroke();
    }

    // Main ground line (bright)
    ctx.set_stroke_style_str("#444466");
    ctx.set_line_width(2.0);
    ctx.begin_path();
    ctx.move_to(0.0, GROUND_Y);
    ctx.line_to(CANVAS_W, GROUND_Y);
    ctx.stroke();

    // Subtle glow along ground line
    let glow = ctx.create_linear_gradient(0.0, GROUND_Y - 3.0, 0.0, GROUND_Y + 3.0);
    let _ = glow.add_color_stop(0.0, "rgba(80,80,140,0.0)");
    let _ = glow.add_color_stop(0.5, "rgba(80,80,140,0.15)");
    let _ = glow.add_color_stop(1.0, "rgba(80,80,140,0.0)");
    ctx.set_fill_style_canvas_gradient(&glow);
    ctx.fill_rect(0.0, GROUND_Y - 3.0, CANVAS_W, 6.0);
}

// ── Character rendering ──────────────────────────────────────────────

fn draw_character(
    ctx: &CanvasRenderingContext2d,
    _player: &PlayerState,
    skeleton: &crate::animation::Skeleton,
    facing: f32,
    flash: bool,
    _player_index: usize,
) {
    // VoidDash: character is invisible during pre-teleport frames (0-4)
    if _player.action == PlayerAction::VoidDash && _player.action_frame < 5 {
        // Draw a fading shadow silhouette
        ctx.save();
        ctx.set_global_alpha(0.2);
        let joints = &skeleton.joints;
        let hips = &joints[JointId::Hips as usize];
        ctx.set_fill_style_str("rgba(80,0,160,0.3)");
        ctx.begin_path();
        let _ = ctx.arc(hips.x as f64, (hips.y - 30.0) as f64, 25.0, 0.0, std::f64::consts::TAU);
        ctx.fill();
        ctx.restore();
        return;
    }

    let joints = &skeleton.joints;
    let accent = if flash { "#ffffff" } else { element_accent(_player.element) };
    let glow = element_glow(_player.element);

    // Ground shadow (ellipse approximated with arc + scale)
    let hips = &joints[JointId::Hips as usize];
    ctx.save();
    ctx.set_fill_style_str("rgba(0,0,0,0.3)");
    ctx.translate(hips.x as f64, GROUND_Y).ok();
    ctx.scale(1.0, 0.2).ok();
    ctx.begin_path();
    let _ = ctx.arc(0.0, 0.0, 24.0, 0.0, std::f64::consts::TAU);
    ctx.fill();
    ctx.restore();

    // ── Body silhouette: thick dark limbs with round caps ──
    let body_color = if flash { "#333333" } else { "#080810" };
    ctx.set_stroke_style_str(body_color);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");

    // Torso / spine
    draw_thick_bone(ctx, joints, JointId::Hips, JointId::Torso, 16.0);
    draw_thick_bone(ctx, joints, JointId::Torso, JointId::Neck, 13.0);

    // Shoulder span
    draw_thick_bone(ctx, joints, JointId::Neck, JointId::LShoulder, 9.0);
    draw_thick_bone(ctx, joints, JointId::Neck, JointId::RShoulder, 9.0);

    // Arms
    draw_thick_bone(ctx, joints, JointId::LShoulder, JointId::LElbow, 7.0);
    draw_thick_bone(ctx, joints, JointId::RShoulder, JointId::RElbow, 7.0);
    draw_thick_bone(ctx, joints, JointId::LElbow, JointId::LWrist, 5.0);
    draw_thick_bone(ctx, joints, JointId::RElbow, JointId::RWrist, 5.0);

    // Legs
    draw_thick_bone(ctx, joints, JointId::Hips, JointId::LKnee, 10.0);
    draw_thick_bone(ctx, joints, JointId::Hips, JointId::RKnee, 10.0);
    draw_thick_bone(ctx, joints, JointId::LKnee, JointId::LAnkle, 7.0);
    draw_thick_bone(ctx, joints, JointId::RKnee, JointId::RAnkle, 7.0);

    // Head (filled dark circle)
    let head = &joints[JointId::Head as usize];
    ctx.begin_path();
    let _ = ctx.arc(head.x as f64, head.y as f64, 9.0, 0.0, std::f64::consts::TAU);
    ctx.set_fill_style_str(body_color);
    ctx.fill();

    // ── Element-colored skeleton overlay with glow ──
    ctx.save();
    ctx.set_shadow_blur(10.0);
    ctx.set_shadow_color(glow);
    ctx.set_stroke_style_str(accent);
    ctx.set_line_width(1.5);
    ctx.set_line_cap("round");

    draw_bone(ctx, joints, JointId::Hips, JointId::Torso);
    draw_bone(ctx, joints, JointId::Torso, JointId::Neck);
    draw_bone(ctx, joints, JointId::Neck, JointId::Head);
    draw_bone(ctx, joints, JointId::Torso, JointId::LShoulder);
    draw_bone(ctx, joints, JointId::LShoulder, JointId::LElbow);
    draw_bone(ctx, joints, JointId::LElbow, JointId::LWrist);
    draw_bone(ctx, joints, JointId::Torso, JointId::RShoulder);
    draw_bone(ctx, joints, JointId::RShoulder, JointId::RElbow);
    draw_bone(ctx, joints, JointId::RElbow, JointId::RWrist);
    draw_bone(ctx, joints, JointId::Hips, JointId::LKnee);
    draw_bone(ctx, joints, JointId::LKnee, JointId::LAnkle);
    draw_bone(ctx, joints, JointId::Hips, JointId::RKnee);
    draw_bone(ctx, joints, JointId::RKnee, JointId::RAnkle);

    // Head outline
    ctx.begin_path();
    let _ = ctx.arc(head.x as f64, head.y as f64, 9.0, 0.0, std::f64::consts::TAU);
    ctx.stroke();

    ctx.restore();

    // ── Glowing eyes ──
    let eye_y = head.y - 1.0;
    let eye_dx = 2.5 * facing;
    ctx.save();
    ctx.set_shadow_blur(8.0);
    ctx.set_shadow_color(accent);
    ctx.set_fill_style_str(accent);
    ctx.begin_path();
    let _ = ctx.arc(
        (head.x + eye_dx - 2.0) as f64,
        eye_y as f64,
        1.3,
        0.0,
        std::f64::consts::TAU,
    );
    ctx.fill();
    ctx.begin_path();
    let _ = ctx.arc(
        (head.x + eye_dx + 2.0) as f64,
        eye_y as f64,
        1.3,
        0.0,
        std::f64::consts::TAU,
    );
    ctx.fill();
    ctx.restore();

    // ── Dual daggers ──
    draw_dagger(ctx, joints, JointId::LWrist, JointId::LElbow, accent);
    draw_dagger(ctx, joints, JointId::RWrist, JointId::RElbow, accent);
}

fn draw_thick_bone(
    ctx: &CanvasRenderingContext2d,
    joints: &[Joint],
    from: JointId,
    to: JointId,
    width: f64,
) {
    let a = &joints[from as usize];
    let b = &joints[to as usize];
    ctx.set_line_width(width);
    ctx.begin_path();
    ctx.move_to(a.x as f64, a.y as f64);
    ctx.line_to(b.x as f64, b.y as f64);
    ctx.stroke();
}

fn draw_bone(
    ctx: &CanvasRenderingContext2d,
    joints: &[Joint],
    from: JointId,
    to: JointId,
) {
    let a = &joints[from as usize];
    let b = &joints[to as usize];
    ctx.begin_path();
    ctx.move_to(a.x as f64, a.y as f64);
    ctx.line_to(b.x as f64, b.y as f64);
    ctx.stroke();
}

fn draw_dagger(
    ctx: &CanvasRenderingContext2d,
    joints: &[Joint],
    wrist: JointId,
    elbow: JointId,
    color: &str,
) {
    let w = &joints[wrist as usize];
    let e = &joints[elbow as usize];
    let dx = w.x - e.x;
    let dy = w.y - e.y;
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let nx = dx / len;
    let ny = dy / len;

    // Perpendicular direction
    let px = -ny;
    let py = nx;

    // Blade tip
    let tip_x = w.x + nx * 22.0;
    let tip_y = w.y + ny * 22.0;

    // Blade width at base
    let bw = 2.5_f32;

    // Filled blade (triangle)
    ctx.begin_path();
    ctx.move_to((w.x + px * bw) as f64, (w.y + py * bw) as f64);
    ctx.line_to(tip_x as f64, tip_y as f64);
    ctx.line_to((w.x - px * bw) as f64, (w.y - py * bw) as f64);
    ctx.close_path();
    ctx.set_fill_style_str(color);
    ctx.fill();

    // Blade center line (bright edge)
    ctx.save();
    ctx.set_shadow_blur(4.0);
    ctx.set_shadow_color(color);
    ctx.set_stroke_style_str(color);
    ctx.set_line_width(1.0);
    ctx.begin_path();
    ctx.move_to(w.x as f64, w.y as f64);
    ctx.line_to(tip_x as f64, tip_y as f64);
    ctx.stroke();
    ctx.restore();

    // Crossguard
    let gx = w.x - nx * 2.0;
    let gy = w.y - ny * 2.0;
    ctx.set_stroke_style_str("#666666");
    ctx.set_line_width(2.5);
    ctx.begin_path();
    ctx.move_to((gx + px * 5.0) as f64, (gy + py * 5.0) as f64);
    ctx.line_to((gx - px * 5.0) as f64, (gy - py * 5.0) as f64);
    ctx.stroke();
}

// ── UI elements ──────────────────────────────────────────────────────

fn draw_health_bar(ctx: &CanvasRenderingContext2d, player: &PlayerState, player_index: usize) {
    let bar_w: f64 = 450.0;
    let bar_h: f64 = 20.0;
    let y: f64 = 24.0;
    let flip = player_index == 1;
    let x: f64 = if flip { CANVAS_W - 30.0 - bar_w } else { 30.0 };

    let ratio = (player.health as f64 / MAX_HEALTH as f64).clamp(0.0, 1.0);

    // Background
    ctx.set_fill_style_str("#1a1a1a");
    ctx.fill_rect(x, y, bar_w, bar_h);

    // Health fill with gradient
    let fill_w = bar_w * ratio;
    let fill_x = if flip { x + bar_w - fill_w } else { x };

    let grad = if flip {
        ctx.create_linear_gradient(fill_x, y, fill_x + fill_w, y)
    } else {
        ctx.create_linear_gradient(fill_x, y, fill_x + fill_w, y)
    };

    if ratio > 0.5 {
        let _ = grad.add_color_stop(0.0, "#22cc22");
        let _ = grad.add_color_stop(1.0, "#33ff33");
    } else if ratio > 0.25 {
        let _ = grad.add_color_stop(0.0, "#ccaa22");
        let _ = grad.add_color_stop(1.0, "#ffcc33");
    } else {
        let _ = grad.add_color_stop(0.0, "#cc2222");
        let _ = grad.add_color_stop(1.0, "#ff4444");
    }

    ctx.set_fill_style_canvas_gradient(&grad);
    ctx.fill_rect(fill_x, y, fill_w, bar_h);

    // Border
    ctx.set_stroke_style_str("#555555");
    ctx.set_line_width(1.0);
    ctx.stroke_rect(x, y, bar_w, bar_h);

    // Player label
    let label_accent = element_accent(player.element);
    ctx.set_fill_style_str(label_accent);
    ctx.set_font("bold 14px monospace");
    if flip {
        ctx.set_text_align("right");
        let _ = ctx.fill_text("P2", CANVAS_W - 30.0, y - 4.0);
    } else {
        ctx.set_text_align("left");
        let _ = ctx.fill_text("P1", 30.0, y - 4.0);
    }
}

pub fn draw_energy_bar(
    ctx: &CanvasRenderingContext2d,
    energy: i32,
    x: f32,
    y: f32,
    flip: bool,
) {
    let bar_w: f32 = 300.0;
    let bar_h: f32 = 8.0;
    let ratio = (energy as f32 / MAX_ENERGY as f32).clamp(0.0, 1.0);

    let bx = if flip { x - bar_w } else { x };

    ctx.set_fill_style_str("#111111");
    ctx.fill_rect(bx as f64, y as f64, bar_w as f64, bar_h as f64);

    let fill_w = bar_w * ratio;
    let fill_x = if flip { bx + bar_w - fill_w } else { bx };
    ctx.set_fill_style_str("#3399ff");
    ctx.fill_rect(fill_x as f64, y as f64, fill_w as f64, bar_h as f64);

    ctx.set_stroke_style_str("#333333");
    ctx.set_line_width(1.0);
    ctx.stroke_rect(bx as f64, y as f64, bar_w as f64, bar_h as f64);
}

pub fn draw_timer(ctx: &CanvasRenderingContext2d, round_timer: i32) {
    let remaining = (round_timer / TICKS_PER_SECOND).max(0);

    // Urgency coloring
    let color = if remaining <= 10 {
        "#ff3333"
    } else if remaining <= 20 {
        "#ffcc00"
    } else {
        "#ffffff"
    };

    ctx.set_fill_style_str(color);
    ctx.set_font("bold 32px monospace");
    ctx.set_text_align("center");
    let text = format!("{}", remaining);
    let _ = ctx.fill_text(&text, CANVAS_W / 2.0, 45.0);
}

pub fn draw_round_indicators(ctx: &CanvasRenderingContext2d, scores: &[i32; 2], elements: &[Element; 2]) {
    let center_x = CANVAS_W / 2.0;
    let y = 62.0;
    let dot_r = 5.0;
    let gap = 14.0;

    // P1 win dots (left of center)
    for i in 0..2 {
        let x = center_x - 30.0 - (i as f64) * gap;
        ctx.begin_path();
        let _ = ctx.arc(x, y, dot_r, 0.0, std::f64::consts::TAU);
        if (i as i32) < scores[0] {
            ctx.set_fill_style_str(element_accent(elements[0]));
        } else {
            ctx.set_fill_style_str("#333333");
        }
        ctx.fill();
        ctx.set_stroke_style_str("#555555");
        ctx.set_line_width(1.0);
        ctx.stroke();
    }

    // P2 win dots (right of center)
    for i in 0..2 {
        let x = center_x + 30.0 + (i as f64) * gap;
        ctx.begin_path();
        let _ = ctx.arc(x, y, dot_r, 0.0, std::f64::consts::TAU);
        if (i as i32) < scores[1] {
            ctx.set_fill_style_str(element_accent(elements[1]));
        } else {
            ctx.set_fill_style_str("#333333");
        }
        ctx.fill();
        ctx.set_stroke_style_str("#555555");
        ctx.set_line_width(1.0);
        ctx.stroke();
    }
}

pub fn draw_combo_counter(ctx: &CanvasRenderingContext2d, state: &GameState) {
    for (i, combo) in state.combo.iter().enumerate() {
        if combo.hit_count > 1 {
            let x = if i == 0 { 150.0 } else { CANVAS_W - 150.0 };
            let align = if i == 0 { "left" } else { "right" };
            ctx.set_text_align(align);
            ctx.set_fill_style_str("#ffcc00");
            ctx.set_font("bold 24px monospace");
            let text = format!("{} HITS!", combo.hit_count);
            let _ = ctx.fill_text(&text, x, CANVAS_H - 50.0);
        }
    }
}

pub fn render_projectiles(ctx: &CanvasRenderingContext2d, projectiles: &[Projectile; MAX_PROJECTILES]) {
    for proj in projectiles {
        if !proj.active {
            continue;
        }
        let px = proj.x.to_f32() as f64;
        let py = proj.y.to_f32() as f64;
        let (color, glow_color) = match proj.element {
            Element::Fire => ("#ff6600", "rgba(255,102,0,0.6)"),
            Element::Lightning => ("#88ccff", "rgba(100,180,255,0.6)"),
            Element::DarkMagic => ("#9933ff", "rgba(153,51,255,0.6)"),
            Element::Ice => ("#66eeff", "rgba(102,238,255,0.6)"),
        };

        // Shadow Surge void orb: larger, darker core with pulsing outer ring
        let is_void_orb = matches!(proj.element, Element::DarkMagic);
        let (outer_r, inner_r) = if is_void_orb { (11.0, 5.0) } else { (8.0, 3.0) };

        // Outer glow
        ctx.save();
        ctx.set_shadow_blur(if is_void_orb { 20.0 } else { 15.0 });
        ctx.set_shadow_color(glow_color);
        ctx.set_fill_style_str(color);
        ctx.begin_path();
        let _ = ctx.arc(px, py, outer_r, 0.0, std::f64::consts::TAU);
        ctx.fill();

        // Inner core — dark for void orb, white for others
        if is_void_orb {
            ctx.set_fill_style_str("#1a0033");
        } else {
            ctx.set_fill_style_str("#ffffff");
        }
        ctx.begin_path();
        let _ = ctx.arc(px, py, inner_r, 0.0, std::f64::consts::TAU);
        ctx.fill();
        ctx.restore();
    }
}

pub fn render_debug_overlay(
    ctx: &CanvasRenderingContext2d,
    state: &GameState,
    p1_input: game_sim::input::Input,
    p2_input: game_sim::input::Input,
) {
    let active_projectiles = state.projectiles.iter().filter(|p| p.active).count();

    fn action_name(p: &PlayerState) -> &'static str {
        use game_sim::player::PlayerAction::*;
        match p.action {
            Idle => "Idle",
            WalkForward => "WalkFwd",
            WalkBack => "WalkBack",
            Jump => "Jump",
            Crouch => "Crouch",
            LightAttack1 => "Light1",
            LightAttack2 => "Light2",
            LightAttack3 => "Light3",
            HeavyAttack => "Heavy",
            Uppercut => "Uppercut",
            AerialAttack => "Aerial",
            Block => "Block",
            Fireball => "Fireball",
            DashStrike => "DashStrike",
            ShadowSurge => "ShadowSurge",
            VoidDash => "VoidDash",
            Hitstun { .. } => "Hitstun",
            Blockstun { .. } => "Blockstun",
            Knockdown { .. } => "Knockdown",
            Getup => "Getup",
        }
    }

    fn input_str(input: game_sim::input::Input) -> String {
        let mut parts = Vec::new();
        if input.is_left() { parts.push("L"); }
        if input.is_right() { parts.push("R"); }
        if input.is_up() { parts.push("U"); }
        if input.is_down() { parts.push("D"); }
        if input.is_light() { parts.push("lt"); }
        if input.is_heavy() { parts.push("hv"); }
        if input.is_special() { parts.push("sp"); }
        if input.is_block() { parts.push("bl"); }
        if parts.is_empty() { return format!("0x{:02X} (none)", input.0); }
        format!("0x{:02X} ({})", input.0, parts.join("+"))
    }

    // Semi-transparent background panel
    ctx.save();
    ctx.set_fill_style_str("rgba(0,0,0,0.75)");
    ctx.fill_rect(0.0, 80.0, 320.0, 170.0);

    ctx.set_font("12px monospace");
    ctx.set_text_align("left");
    ctx.set_fill_style_str("#00ff88");

    let lines = [
        format!("F1 DEBUG  frame:{}", state.frame_number),
        format!("P1 input: {}", input_str(p1_input)),
        format!("P1 state: {} f:{} energy:{} hp:{}",
            action_name(&state.players[0]),
            state.players[0].action_frame,
            state.players[0].energy,
            state.players[0].health),
        format!("P2 input: {}", input_str(p2_input)),
        format!("P2 state: {} f:{} energy:{} hp:{}",
            action_name(&state.players[1]),
            state.players[1].action_frame,
            state.players[1].energy,
            state.players[1].health),
        format!("Projectiles: {}/{}", active_projectiles, MAX_PROJECTILES),
        format!("P1 pos: ({},{}) grounded:{}",
            state.players[0].x.to_f32() as i32,
            state.players[0].y.to_f32() as i32,
            state.players[0].is_grounded),
        format!("P2 pos: ({},{}) grounded:{}",
            state.players[1].x.to_f32() as i32,
            state.players[1].y.to_f32() as i32,
            state.players[1].is_grounded),
    ];

    for (i, line) in lines.iter().enumerate() {
        let _ = ctx.fill_text(line, 8.0, 96.0 + i as f64 * 18.0);
    }

    ctx.restore();
}
