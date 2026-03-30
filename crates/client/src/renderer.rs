use game_sim::constants::{MAX_ENERGY, MAX_HEALTH, TICKS_PER_SECOND};
use game_sim::player::{Element, PlayerState};
use game_sim::GameState;
use web_sys::CanvasRenderingContext2d;

use crate::animation::{compute_skeleton, get_animation, AnimationState, JointId, Skeleton};

const CANVAS_W: f64 = 1200.0;
const CANVAS_H: f64 = 600.0;
const GROUND_Y: f64 = 500.0;

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
        let skeleton = compute_skeleton(
            p.x.to_f32(),
            p.y.to_f32(),
            facing,
            &anim,
            anim_state.frame,
        );
        let flash = hit_flash[i] > 0;
        draw_character(ctx, p, &skeleton, facing, flash);
    }

    // UI
    draw_health_bar(ctx, state.players[0].health, MAX_HEALTH, 30.0, 30.0, false);
    draw_health_bar(ctx, state.players[1].health, MAX_HEALTH, CANVAS_W as f32 - 30.0, 30.0, true);
    draw_energy_bar(ctx, state.players[0].energy, 30.0, 55.0, false);
    draw_energy_bar(ctx, state.players[1].energy, CANVAS_W as f32 - 30.0, 55.0, true);
    draw_timer(ctx, state.round_timer);
    draw_round_counter(ctx, &state.round_scores);
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

fn element_color(element: Element) -> &'static str {
    match element {
        Element::Fire => "#ff6600",
        Element::Lightning => "#aaccff",
        Element::DarkMagic => "#9933ff",
        Element::Ice => "#66eeff",
    }
}

fn draw_character(
    ctx: &CanvasRenderingContext2d,
    player: &PlayerState,
    skeleton: &Skeleton,
    facing: f32,
    flash: bool,
) {
    let base_color = element_color(player.element);
    let color = if flash { "#ffffff" } else { base_color };
    let joints = &skeleton.joints;

    // Silhouette outline path
    let outline_indices = [
        JointId::Head as usize,
        JointId::Neck as usize,
        JointId::LShoulder as usize,
        JointId::LElbow as usize,
        JointId::LWrist as usize,
        JointId::LElbow as usize,
        JointId::LShoulder as usize,
        JointId::Torso as usize,
        JointId::Hips as usize,
        JointId::LKnee as usize,
        JointId::LAnkle as usize,
        JointId::LKnee as usize,
        JointId::Hips as usize,
        JointId::RKnee as usize,
        JointId::RAnkle as usize,
        JointId::RKnee as usize,
        JointId::Hips as usize,
        JointId::Torso as usize,
        JointId::RShoulder as usize,
        JointId::RElbow as usize,
        JointId::RWrist as usize,
        JointId::RElbow as usize,
        JointId::RShoulder as usize,
        JointId::Neck as usize,
    ];

    ctx.begin_path();
    let first = &joints[outline_indices[0]];
    ctx.move_to(first.x as f64, first.y as f64);
    for &idx in &outline_indices[1..] {
        ctx.line_to(joints[idx].x as f64, joints[idx].y as f64);
    }
    ctx.close_path();
    let fill_color = if flash { "#333333" } else { "#0a0a0a" };
    ctx.set_fill_style_str(fill_color);
    ctx.fill();

    // Skeleton bones
    ctx.set_stroke_style_str(color);
    ctx.set_line_width(2.0);

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

    // Daggers
    draw_dagger(ctx, joints, JointId::LWrist, JointId::LElbow, facing, color);
    draw_dagger(ctx, joints, JointId::RWrist, JointId::RElbow, facing, color);

    // Head circle
    let head = &joints[JointId::Head as usize];
    ctx.begin_path();
    let _ = ctx.arc(head.x as f64, head.y as f64, 6.0, 0.0, std::f64::consts::TAU);
    ctx.set_fill_style_str("#0a0a0a");
    ctx.fill();
    ctx.set_stroke_style_str(color);
    ctx.stroke();
}

fn draw_bone(
    ctx: &CanvasRenderingContext2d,
    joints: &[crate::animation::Joint],
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
    joints: &[crate::animation::Joint],
    wrist: JointId,
    elbow: JointId,
    _facing: f32,
    color: &str,
) {
    let w = &joints[wrist as usize];
    let e = &joints[elbow as usize];
    let dx = w.x - e.x;
    let dy = w.y - e.y;
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let nx = dx / len;
    let ny = dy / len;
    let tip_x = w.x + nx * 18.0;
    let tip_y = w.y + ny * 18.0;

    ctx.set_stroke_style_str(color);
    ctx.set_line_width(2.5);
    ctx.begin_path();
    ctx.move_to(w.x as f64, w.y as f64);
    ctx.line_to(tip_x as f64, tip_y as f64);
    ctx.stroke();
}

pub fn draw_health_bar(
    ctx: &CanvasRenderingContext2d,
    health: i32,
    max_health: i32,
    x: f32,
    y: f32,
    flip: bool,
) {
    let bar_w: f32 = 450.0;
    let bar_h: f32 = 18.0;
    let ratio = (health as f32 / max_health as f32).clamp(0.0, 1.0);

    let bx = if flip { x - bar_w } else { x };

    ctx.set_fill_style_str("#1a1a1a");
    ctx.fill_rect(bx as f64, y as f64, bar_w as f64, bar_h as f64);

    let fill_w = bar_w * ratio;
    let fill_x = if flip { bx + bar_w - fill_w } else { bx };
    let health_color = if ratio > 0.5 {
        "#33cc33"
    } else if ratio > 0.25 {
        "#cccc33"
    } else {
        "#cc3333"
    };
    ctx.set_fill_style_str(health_color);
    ctx.fill_rect(fill_x as f64, y as f64, fill_w as f64, bar_h as f64);

    ctx.set_stroke_style_str("#555555");
    ctx.set_line_width(1.0);
    ctx.stroke_rect(bx as f64, y as f64, bar_w as f64, bar_h as f64);
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

    ctx.set_fill_style_str("#ffffff");
    ctx.set_font("bold 28px monospace");
    ctx.set_text_align("center");
    let text = format!("{}", remaining);
    let _ = ctx.fill_text(&text, CANVAS_W / 2.0, 45.0);
}

pub fn draw_round_counter(ctx: &CanvasRenderingContext2d, scores: &[i32; 2]) {
    ctx.set_fill_style_str("#aaaaaa");
    ctx.set_font("16px monospace");

    ctx.set_text_align("left");
    let p1 = format!("P1: {}", scores[0]);
    let _ = ctx.fill_text(&p1, 30.0, 85.0);

    ctx.set_text_align("right");
    let p2 = format!("P2: {}", scores[1]);
    let _ = ctx.fill_text(&p2, CANVAS_W - 30.0, 85.0);
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
