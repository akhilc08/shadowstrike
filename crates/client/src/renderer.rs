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
    ctx.set_fill_style_str("#0a0015");
    ctx.fill_rect(0.0, 0.0, CANVAS_W, CANVAS_H);

    draw_background(ctx, state.frame_number);

    // Draw each player
    for (i, anim_state) in anim_states.iter().enumerate() {
        let p = &state.players[i];
        let facing = p.facing as f32;
        let anim = get_animation(anim_state.anim_id);
        let skeleton = compute_skeleton(p.x.to_f32(), p.y.to_f32(), facing, &anim, anim_state.frame);
        let flash = hit_flash[i] > 0;
        draw_character(ctx, p, &skeleton, facing, flash, i, state.frame_number);
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

// ── Stage background: dark fantasy ruins ─────────────────────────────

fn draw_background(ctx: &CanvasRenderingContext2d, frame: u64) {
    let t = frame as f64 * 0.016; // time in pseudo-seconds

    // Sky gradient — deep midnight
    let sky = ctx.create_linear_gradient(0.0, 0.0, 0.0, GROUND_Y);
    let _ = sky.add_color_stop(0.0, "#050008");
    let _ = sky.add_color_stop(0.3, "#0a0015");
    let _ = sky.add_color_stop(0.7, "#0d0030");
    let _ = sky.add_color_stop(1.0, "#1a0a2e");
    ctx.set_fill_style_canvas_gradient(&sky);
    ctx.fill_rect(0.0, 0.0, CANVAS_W, GROUND_Y);

    // Full moon with halo
    let moon_x = 900.0;
    let moon_y = 80.0;
    // Outer halo
    let halo = ctx.create_radial_gradient(moon_x, moon_y, 15.0, moon_x, moon_y, 120.0).unwrap();
    let _ = halo.add_color_stop(0.0, "rgba(200,200,255,0.08)");
    let _ = halo.add_color_stop(0.5, "rgba(102,0,204,0.04)");
    let _ = halo.add_color_stop(1.0, "rgba(10,0,21,0)");
    ctx.set_fill_style_canvas_gradient(&halo);
    ctx.fill_rect(moon_x - 120.0, moon_y - 120.0, 240.0, 240.0);
    // Moon disc
    ctx.set_fill_style_str("rgba(220,215,240,0.9)");
    ctx.begin_path();
    let _ = ctx.arc(moon_x, moon_y, 22.0, 0.0, std::f64::consts::TAU);
    ctx.fill();
    // Moon inner glow
    ctx.save();
    ctx.set_shadow_blur(20.0);
    ctx.set_shadow_color("rgba(200,200,255,0.5)");
    ctx.set_fill_style_str("rgba(240,238,255,0.7)");
    ctx.begin_path();
    let _ = ctx.arc(moon_x, moon_y, 18.0, 0.0, std::f64::consts::TAU);
    ctx.fill();
    ctx.restore();

    // Distant mountains (dark silhouettes)
    ctx.set_fill_style_str("#0a0520");
    ctx.begin_path();
    ctx.move_to(0.0, 350.0);
    ctx.line_to(80.0, 280.0);
    ctx.line_to(180.0, 310.0);
    ctx.line_to(280.0, 260.0);
    ctx.line_to(400.0, 300.0);
    ctx.line_to(500.0, 240.0);
    ctx.line_to(600.0, 270.0);
    ctx.line_to(720.0, 220.0);
    ctx.line_to(850.0, 290.0);
    ctx.line_to(950.0, 250.0);
    ctx.line_to(1050.0, 300.0);
    ctx.line_to(1150.0, 270.0);
    ctx.line_to(1200.0, 320.0);
    ctx.line_to(1200.0, GROUND_Y);
    ctx.line_to(0.0, GROUND_Y);
    ctx.close_path();
    ctx.fill();

    // Mid-ground ruins: broken columns and arches
    ctx.set_fill_style_str("#120830");
    // Left broken column
    ctx.fill_rect(100.0, 330.0, 20.0, GROUND_Y - 330.0);
    ctx.fill_rect(95.0, 325.0, 30.0, 10.0); // Capital
    // Broken top
    ctx.begin_path();
    ctx.move_to(95.0, 325.0);
    ctx.line_to(105.0, 310.0);
    ctx.line_to(115.0, 320.0);
    ctx.line_to(125.0, 315.0);
    ctx.close_path();
    ctx.fill();

    // Right broken column
    ctx.fill_rect(1070.0, 340.0, 22.0, GROUND_Y - 340.0);
    ctx.fill_rect(1065.0, 335.0, 32.0, 10.0);
    ctx.begin_path();
    ctx.move_to(1065.0, 335.0);
    ctx.line_to(1075.0, 318.0);
    ctx.line_to(1085.0, 328.0);
    ctx.line_to(1097.0, 322.0);
    ctx.close_path();
    ctx.fill();

    // Center arch (ruined)
    ctx.set_fill_style_str("#0f0628");
    ctx.fill_rect(540.0, 310.0, 16.0, GROUND_Y - 310.0); // Left pillar
    ctx.fill_rect(644.0, 310.0, 16.0, GROUND_Y - 310.0); // Right pillar
    // Arch top
    ctx.begin_path();
    ctx.move_to(540.0, 310.0);
    let _ = ctx.arc(600.0, 310.0, 60.0, std::f64::consts::PI, 0.0);
    ctx.line_to(660.0, 310.0);
    ctx.line_to(644.0, 310.0);
    let _ = ctx.arc(600.0, 310.0, 44.0, 0.0, std::f64::consts::PI);
    ctx.close_path();
    ctx.fill();
    // Broken chunk missing from arch
    ctx.set_fill_style_str("#0a0015");
    ctx.begin_path();
    ctx.move_to(620.0, 252.0);
    ctx.line_to(640.0, 258.0);
    ctx.line_to(635.0, 270.0);
    ctx.line_to(615.0, 265.0);
    ctx.close_path();
    ctx.fill();

    // Small background columns
    ctx.set_fill_style_str("#0d0425");
    ctx.fill_rect(300.0, 380.0, 12.0, GROUND_Y - 380.0);
    ctx.fill_rect(295.0, 376.0, 22.0, 8.0);
    ctx.fill_rect(880.0, 370.0, 14.0, GROUND_Y - 370.0);
    ctx.fill_rect(875.0, 366.0, 24.0, 8.0);

    // Ground plane — crumbled stone floor
    let floor = ctx.create_linear_gradient(0.0, GROUND_Y, 0.0, CANVAS_H);
    let _ = floor.add_color_stop(0.0, "#2a1a10");
    let _ = floor.add_color_stop(0.3, "#1a1008");
    let _ = floor.add_color_stop(1.0, "#0a0505");
    ctx.set_fill_style_canvas_gradient(&floor);
    ctx.fill_rect(0.0, GROUND_Y, CANVAS_W, CANVAS_H - GROUND_Y);

    // Ground line with gold highlight
    ctx.set_stroke_style_str("#4a3a20");
    ctx.set_line_width(2.0);
    ctx.begin_path();
    ctx.move_to(0.0, GROUND_Y);
    ctx.line_to(CANVAS_W, GROUND_Y);
    ctx.stroke();

    // Ground glow
    let glow = ctx.create_linear_gradient(0.0, GROUND_Y - 3.0, 0.0, GROUND_Y + 4.0);
    let _ = glow.add_color_stop(0.0, "rgba(204,153,0,0.0)");
    let _ = glow.add_color_stop(0.5, "rgba(204,153,0,0.08)");
    let _ = glow.add_color_stop(1.0, "rgba(204,153,0,0.0)");
    ctx.set_fill_style_canvas_gradient(&glow);
    ctx.fill_rect(0.0, GROUND_Y - 3.0, CANVAS_W, 7.0);

    // Cracked stone lines on floor
    ctx.set_stroke_style_str("rgba(60,40,20,0.3)");
    ctx.set_line_width(1.0);
    let cracks: &[(f64, f64, f64, f64)] = &[
        (150.0, GROUND_Y + 10.0, 200.0, GROUND_Y + 40.0),
        (400.0, GROUND_Y + 5.0, 380.0, GROUND_Y + 50.0),
        (700.0, GROUND_Y + 8.0, 750.0, GROUND_Y + 45.0),
        (950.0, GROUND_Y + 12.0, 920.0, GROUND_Y + 55.0),
        (550.0, GROUND_Y + 3.0, 600.0, GROUND_Y + 30.0),
    ];
    for &(x1, y1, x2, y2) in cracks {
        ctx.begin_path();
        ctx.move_to(x1, y1);
        ctx.line_to((x1 + x2) * 0.5 + 8.0, (y1 + y2) * 0.5);
        ctx.line_to(x2, y2);
        ctx.stroke();
    }

    // Torch flames on columns (particle-like procedural fire)
    draw_torch_flame(ctx, 110.0, 322.0, t);
    draw_torch_flame(ctx, 1081.0, 332.0, t + 2.0);

    // Floating arcane sigils (slowly rotating)
    draw_floating_sigil(ctx, 200.0, 400.0, t * 0.3, "rgba(153,0,255,0.06)");
    draw_floating_sigil(ctx, 800.0, 420.0, t * 0.25 + 1.0, "rgba(255,102,0,0.05)");
    draw_floating_sigil(ctx, 500.0, 440.0, t * 0.2 + 2.5, "rgba(255,215,0,0.04)");

    // Ground fog (rolling wisps)
    draw_ground_fog(ctx, t);
}

fn draw_torch_flame(ctx: &CanvasRenderingContext2d, x: f64, y: f64, t: f64) {
    // Flickering flame made of layered circles
    for i in 0..5 {
        let fi = i as f64;
        let flicker = (t * 8.0 + fi * 1.7).sin() * 3.0;
        let flicker2 = (t * 12.0 + fi * 2.3).cos() * 2.0;
        let fy = y - fi * 4.0 + flicker * 0.5;
        let fx = x + flicker2 * 0.5;
        let size = 5.0 - fi * 0.8;
        let alpha = 0.5 - fi * 0.08;
        if i < 2 {
            ctx.set_fill_style_str(&format!("rgba(255,102,0,{:.2})", alpha));
        } else {
            ctx.set_fill_style_str(&format!("rgba(255,200,50,{:.2})", alpha * 0.7));
        }
        ctx.begin_path();
        let _ = ctx.arc(fx, fy, size, 0.0, std::f64::consts::TAU);
        ctx.fill();
    }
    // Glow
    ctx.save();
    ctx.set_shadow_blur(15.0);
    ctx.set_shadow_color("rgba(255,102,0,0.3)");
    ctx.set_fill_style_str("rgba(255,150,50,0.15)");
    ctx.begin_path();
    let _ = ctx.arc(x, y - 8.0, 8.0, 0.0, std::f64::consts::TAU);
    ctx.fill();
    ctx.restore();
}

fn draw_floating_sigil(ctx: &CanvasRenderingContext2d, x: f64, y: f64, angle: f64, color: &str) {
    ctx.save();
    ctx.translate(x, y).ok();
    ctx.rotate(angle).ok();
    ctx.set_stroke_style_str(color);
    ctx.set_line_width(1.5);

    // Outer circle
    ctx.begin_path();
    let _ = ctx.arc(0.0, 0.0, 20.0, 0.0, std::f64::consts::TAU);
    ctx.stroke();

    // Inner triangle
    ctx.begin_path();
    for i in 0..3 {
        let a = (i as f64 / 3.0) * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
        let px = a.cos() * 14.0;
        let py = a.sin() * 14.0;
        if i == 0 { ctx.move_to(px, py); } else { ctx.line_to(px, py); }
    }
    ctx.close_path();
    ctx.stroke();

    // Center dot
    ctx.set_fill_style_str(color);
    ctx.begin_path();
    let _ = ctx.arc(0.0, 0.0, 2.0, 0.0, std::f64::consts::TAU);
    ctx.fill();

    ctx.restore();
}

fn draw_ground_fog(ctx: &CanvasRenderingContext2d, t: f64) {
    // Rolling fog wisps along the ground
    for i in 0..6 {
        let fi = i as f64;
        let base_x = ((t * 15.0 + fi * 220.0) % (CANVAS_W + 300.0)) - 150.0;
        let y = GROUND_Y + 10.0 + fi * 8.0;
        let w = 180.0 + fi * 30.0;
        let h = 20.0 + fi * 5.0;
        let alpha = 0.03 + (t * 0.5 + fi).sin().abs() * 0.02;

        let fog = ctx.create_radial_gradient(base_x, y, 0.0, base_x, y, w * 0.5).unwrap();
        let _ = fog.add_color_stop(0.0, &format!("rgba(102,0,204,{:.3})", alpha));
        let _ = fog.add_color_stop(1.0, "rgba(10,0,21,0)");
        ctx.set_fill_style_canvas_gradient(&fog);
        ctx.fill_rect(base_x - w * 0.5, y - h * 0.5, w, h);
    }
}

// ── Character rendering ──────────────────────────────────────────────

fn draw_character(
    ctx: &CanvasRenderingContext2d,
    _player: &PlayerState,
    skeleton: &crate::animation::Skeleton,
    facing: f32,
    flash: bool,
    _player_index: usize,
    frame: u64,
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
    let t = frame as f64 * 0.016;

    // Ground shadow (ellipse)
    let hips = &joints[JointId::Hips as usize];
    ctx.save();
    ctx.set_fill_style_str("rgba(0,0,0,0.35)");
    ctx.translate(hips.x as f64, GROUND_Y).ok();
    ctx.scale(1.0, 0.2).ok();
    ctx.begin_path();
    let _ = ctx.arc(0.0, 0.0, 26.0, 0.0, std::f64::consts::TAU);
    ctx.fill();
    ctx.restore();

    // Element-specific aura beneath character
    match _player.element {
        Element::Fire => {
            // Faint fire halo
            ctx.save();
            let pulse = (t * 3.0).sin() * 0.03 + 0.08;
            let halo = ctx.create_radial_gradient(
                hips.x as f64, (hips.y - 20.0) as f64, 5.0,
                hips.x as f64, (hips.y - 20.0) as f64, 50.0,
            ).unwrap();
            let _ = halo.add_color_stop(0.0, &format!("rgba(255,102,0,{:.3})", pulse));
            let _ = halo.add_color_stop(1.0, "rgba(255,102,0,0)");
            ctx.set_fill_style_canvas_gradient(&halo);
            ctx.fill_rect(
                hips.x as f64 - 50.0, hips.y as f64 - 70.0, 100.0, 100.0,
            );
            ctx.restore();
        }
        Element::Lightning => {
            // Electric arc crackles (drawn as short jagged lines)
            if frame % 4 < 2 {
                ctx.save();
                ctx.set_stroke_style_str("rgba(136,204,255,0.25)");
                ctx.set_line_width(0.8);
                let seed = frame as f32;
                for i in 0..3 {
                    let fi = i as f32;
                    let start_joint = match i {
                        0 => JointId::LShoulder,
                        1 => JointId::RShoulder,
                        _ => JointId::Head,
                    };
                    let j = &joints[start_joint as usize];
                    let dx = pseudo_sin((seed + fi * 37.0) * 0.1) * 15.0;
                    let dy = pseudo_sin((seed + fi * 53.0) * 0.13) * 12.0;
                    ctx.begin_path();
                    ctx.move_to(j.x as f64, j.y as f64);
                    let mid_x = j.x as f64 + dx as f64 * 0.5;
                    let mid_y = j.y as f64 + dy as f64 * 0.5 + 3.0;
                    ctx.line_to(mid_x, mid_y);
                    ctx.line_to(j.x as f64 + dx as f64, j.y as f64 + dy as f64);
                    ctx.stroke();
                }
                ctx.restore();
            }
        }
        Element::DarkMagic => {
            // Dark void wisps orbiting the body
            ctx.save();
            let cx = hips.x as f64;
            let cy = (hips.y - 25.0) as f64;
            for i in 0..4 {
                let fi = i as f64;
                let angle = t * 1.5 + fi * std::f64::consts::FRAC_PI_2;
                let r = 22.0 + (t * 2.0 + fi).sin() * 5.0;
                let wx = cx + angle.cos() * r;
                let wy = cy + angle.sin() * r * 0.6;
                let alpha = 0.15 + (t * 3.0 + fi * 2.0).sin().abs() * 0.1;
                ctx.set_fill_style_str(&format!("rgba(80,0,160,{:.2})", alpha));
                ctx.begin_path();
                let _ = ctx.arc(wx, wy, 3.0, 0.0, std::f64::consts::TAU);
                ctx.fill();
            }
            ctx.restore();
        }
        Element::Ice => {
            // Frosty shimmer
            ctx.save();
            let pulse = (t * 2.5).sin() * 0.02 + 0.06;
            let halo = ctx.create_radial_gradient(
                hips.x as f64, (hips.y - 20.0) as f64, 5.0,
                hips.x as f64, (hips.y - 20.0) as f64, 45.0,
            ).unwrap();
            let _ = halo.add_color_stop(0.0, &format!("rgba(102,238,255,{:.3})", pulse));
            let _ = halo.add_color_stop(1.0, "rgba(102,238,255,0)");
            ctx.set_fill_style_canvas_gradient(&halo);
            ctx.fill_rect(
                hips.x as f64 - 45.0, hips.y as f64 - 65.0, 90.0, 90.0,
            );
            ctx.restore();
        }
    }

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
    ctx.set_shadow_blur(12.0);
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
    ctx.set_shadow_blur(10.0);
    ctx.set_shadow_color(accent);
    ctx.set_fill_style_str(accent);
    ctx.begin_path();
    let _ = ctx.arc(
        (head.x + eye_dx - 2.0) as f64,
        eye_y as f64,
        1.5,
        0.0,
        std::f64::consts::TAU,
    );
    ctx.fill();
    ctx.begin_path();
    let _ = ctx.arc(
        (head.x + eye_dx + 2.0) as f64,
        eye_y as f64,
        1.5,
        0.0,
        std::f64::consts::TAU,
    );
    ctx.fill();
    ctx.restore();

    // ── Dual daggers ──
    draw_dagger(ctx, joints, JointId::LWrist, JointId::LElbow, accent);
    draw_dagger(ctx, joints, JointId::RWrist, JointId::RElbow, accent);
}

/// Simple sin approximation for use in rendering (avoids f32::sin import issues).
fn pseudo_sin(x: f32) -> f32 {
    (x as f64).sin() as f32
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

    // Crossguard (gold trim)
    let gx = w.x - nx * 2.0;
    let gy = w.y - ny * 2.0;
    ctx.set_stroke_style_str("#cc9900");
    ctx.set_line_width(2.5);
    ctx.begin_path();
    ctx.move_to((gx + px * 5.0) as f64, (gy + py * 5.0) as f64);
    ctx.line_to((gx - px * 5.0) as f64, (gy - py * 5.0) as f64);
    ctx.stroke();
}

// ── UI elements (dark fantasy HUD) ──────────────────────────────────

fn draw_health_bar(ctx: &CanvasRenderingContext2d, player: &PlayerState, player_index: usize) {
    let bar_w: f64 = 440.0;
    let bar_h: f64 = 22.0;
    let y: f64 = 26.0;
    let flip = player_index == 1;
    let x: f64 = if flip { CANVAS_W - 30.0 - bar_w } else { 30.0 };

    let ratio = (player.health as f64 / MAX_HEALTH as f64).clamp(0.0, 1.0);

    // Ornate gold border frame
    ctx.set_stroke_style_str("#cc9900");
    ctx.set_line_width(2.0);
    ctx.stroke_rect(x - 1.0, y - 1.0, bar_w + 2.0, bar_h + 2.0);

    // Outer gold glow
    ctx.save();
    ctx.set_shadow_blur(6.0);
    ctx.set_shadow_color("rgba(204,153,0,0.2)");
    ctx.set_stroke_style_str("#ffd700");
    ctx.set_line_width(0.5);
    ctx.stroke_rect(x - 2.0, y - 2.0, bar_w + 4.0, bar_h + 4.0);
    ctx.restore();

    // Background (dark stone)
    ctx.set_fill_style_str("#1a0a08");
    ctx.fill_rect(x, y, bar_w, bar_h);

    // Health fill — ember orange to deep red
    let fill_w = bar_w * ratio;
    let fill_x = if flip { x + bar_w - fill_w } else { x };

    let grad = ctx.create_linear_gradient(fill_x, y, fill_x + fill_w, y);
    if ratio > 0.5 {
        let _ = grad.add_color_stop(0.0, "#ff6600");
        let _ = grad.add_color_stop(1.0, "#ff8833");
    } else if ratio > 0.25 {
        let _ = grad.add_color_stop(0.0, "#cc3300");
        let _ = grad.add_color_stop(1.0, "#ff6600");
    } else {
        let _ = grad.add_color_stop(0.0, "#880000");
        let _ = grad.add_color_stop(1.0, "#cc2200");
    }
    ctx.set_fill_style_canvas_gradient(&grad);
    ctx.fill_rect(fill_x, y, fill_w, bar_h);

    // Crack overlay lines on the bar
    ctx.set_stroke_style_str("rgba(30,15,5,0.4)");
    ctx.set_line_width(0.8);
    for i in 0..4 {
        let cx = x + (i as f64 + 1.0) * bar_w / 5.0;
        ctx.begin_path();
        ctx.move_to(cx, y + 2.0);
        ctx.line_to(cx + 3.0, y + bar_h * 0.5);
        ctx.line_to(cx - 2.0, y + bar_h - 2.0);
        ctx.stroke();
    }

    // Gemstone at each end
    let gem_r = 5.0;
    let gem_y = y + bar_h * 0.5;
    // Left gem
    draw_gemstone(ctx, x - 6.0, gem_y, gem_r, element_accent(player.element));
    // Right gem
    draw_gemstone(ctx, x + bar_w + 6.0, gem_y, gem_r, element_accent(player.element));

    // Player label in gothic font
    let label_accent = element_accent(player.element);
    ctx.save();
    ctx.set_shadow_blur(8.0);
    ctx.set_shadow_color(element_glow(player.element));
    ctx.set_fill_style_str(label_accent);
    ctx.set_font("bold 14px 'Cinzel', serif");
    if flip {
        ctx.set_text_align("right");
        let _ = ctx.fill_text("P2", CANVAS_W - 30.0, y - 6.0);
    } else {
        ctx.set_text_align("left");
        let _ = ctx.fill_text("P1", 30.0, y - 6.0);
    }
    ctx.restore();
}

fn draw_gemstone(ctx: &CanvasRenderingContext2d, x: f64, y: f64, r: f64, color: &str) {
    // Diamond shape
    ctx.save();
    ctx.set_shadow_blur(6.0);
    ctx.set_shadow_color(color);
    ctx.set_fill_style_str(color);
    ctx.begin_path();
    ctx.move_to(x, y - r);
    ctx.line_to(x + r * 0.7, y);
    ctx.line_to(x, y + r);
    ctx.line_to(x - r * 0.7, y);
    ctx.close_path();
    ctx.fill();
    // Inner highlight
    ctx.set_fill_style_str("rgba(255,255,255,0.3)");
    ctx.begin_path();
    ctx.move_to(x, y - r * 0.5);
    ctx.line_to(x + r * 0.3, y);
    ctx.line_to(x, y + r * 0.2);
    ctx.line_to(x - r * 0.3, y);
    ctx.close_path();
    ctx.fill();
    // Gold border
    ctx.set_stroke_style_str("#cc9900");
    ctx.set_line_width(1.0);
    ctx.begin_path();
    ctx.move_to(x, y - r);
    ctx.line_to(x + r * 0.7, y);
    ctx.line_to(x, y + r);
    ctx.line_to(x - r * 0.7, y);
    ctx.close_path();
    ctx.stroke();
    ctx.restore();
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

    // Dark background
    ctx.set_fill_style_str("#0a0515");
    ctx.fill_rect(bx as f64, y as f64, bar_w as f64, bar_h as f64);

    // Energy fill (void purple)
    let fill_w = bar_w * ratio;
    let fill_x = if flip { bx + bar_w - fill_w } else { bx };
    let grad = ctx.create_linear_gradient(fill_x as f64, y as f64, (fill_x + fill_w) as f64, y as f64);
    let _ = grad.add_color_stop(0.0, "#6600cc");
    let _ = grad.add_color_stop(1.0, "#9900ff");
    ctx.set_fill_style_canvas_gradient(&grad);
    ctx.fill_rect(fill_x as f64, y as f64, fill_w as f64, bar_h as f64);

    // Gold border
    ctx.set_stroke_style_str("#4a3a20");
    ctx.set_line_width(1.0);
    ctx.stroke_rect(bx as f64, y as f64, bar_w as f64, bar_h as f64);
}

pub fn draw_timer(ctx: &CanvasRenderingContext2d, round_timer: i32) {
    let remaining = (round_timer / TICKS_PER_SECOND).max(0);
    let cx = CANVAS_W / 2.0;
    let cy = 36.0;

    // Gold medallion background
    ctx.save();
    ctx.set_shadow_blur(10.0);
    ctx.set_shadow_color("rgba(255,215,0,0.15)");

    // Medallion circle
    ctx.set_fill_style_str("#1a0a08");
    ctx.begin_path();
    let _ = ctx.arc(cx, cy, 24.0, 0.0, std::f64::consts::TAU);
    ctx.fill();

    // Gold ring
    ctx.set_stroke_style_str("#cc9900");
    ctx.set_line_width(2.0);
    ctx.begin_path();
    let _ = ctx.arc(cx, cy, 24.0, 0.0, std::f64::consts::TAU);
    ctx.stroke();

    // Inner ring
    ctx.set_stroke_style_str("#ffd700");
    ctx.set_line_width(0.5);
    ctx.begin_path();
    let _ = ctx.arc(cx, cy, 20.0, 0.0, std::f64::consts::TAU);
    ctx.stroke();
    ctx.restore();

    // Urgency coloring
    let color = if remaining <= 10 {
        "#ff3333"
    } else if remaining <= 20 {
        "#ffcc00"
    } else {
        "#ffd700"
    };

    ctx.save();
    if remaining <= 10 {
        ctx.set_shadow_blur(8.0);
        ctx.set_shadow_color("rgba(255,51,51,0.4)");
    }
    ctx.set_fill_style_str(color);
    ctx.set_font("bold 22px 'Cinzel', serif");
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let text = format!("{}", remaining);
    let _ = ctx.fill_text(&text, cx, cy + 1.0);
    ctx.set_text_baseline("alphabetic");
    ctx.restore();
}

pub fn draw_round_indicators(ctx: &CanvasRenderingContext2d, scores: &[i32; 2], elements: &[Element; 2]) {
    let center_x = CANVAS_W / 2.0;
    let y = 68.0;
    let dot_r = 6.0;
    let gap = 16.0;

    // P1 win dots (left of center) — arcane circles
    for i in 0..2 {
        let x = center_x - 35.0 - (i as f64) * gap;
        let won = (i as i32) < scores[0];

        // Outer ring
        ctx.begin_path();
        let _ = ctx.arc(x, y, dot_r, 0.0, std::f64::consts::TAU);
        ctx.set_stroke_style_str(if won { "#cc9900" } else { "#3a2a1a" });
        ctx.set_line_width(1.5);
        ctx.stroke();

        if won {
            ctx.save();
            ctx.set_shadow_blur(8.0);
            ctx.set_shadow_color(element_glow(elements[0]));
            ctx.set_fill_style_str(element_accent(elements[0]));
            ctx.begin_path();
            let _ = ctx.arc(x, y, dot_r - 2.0, 0.0, std::f64::consts::TAU);
            ctx.fill();
            ctx.restore();
        }
    }

    // P2 win dots (right of center)
    for i in 0..2 {
        let x = center_x + 35.0 + (i as f64) * gap;
        let won = (i as i32) < scores[1];

        ctx.begin_path();
        let _ = ctx.arc(x, y, dot_r, 0.0, std::f64::consts::TAU);
        ctx.set_stroke_style_str(if won { "#cc9900" } else { "#3a2a1a" });
        ctx.set_line_width(1.5);
        ctx.stroke();

        if won {
            ctx.save();
            ctx.set_shadow_blur(8.0);
            ctx.set_shadow_color(element_glow(elements[1]));
            ctx.set_fill_style_str(element_accent(elements[1]));
            ctx.begin_path();
            let _ = ctx.arc(x, y, dot_r - 2.0, 0.0, std::f64::consts::TAU);
            ctx.fill();
            ctx.restore();
        }
    }
}

pub fn draw_combo_counter(ctx: &CanvasRenderingContext2d, state: &GameState) {
    for (i, combo) in state.combo.iter().enumerate() {
        if combo.hit_count > 1 {
            let x = if i == 0 { 150.0 } else { CANVAS_W - 150.0 };
            let align = if i == 0 { "left" } else { "right" };
            ctx.save();
            ctx.set_shadow_blur(10.0);
            ctx.set_shadow_color("rgba(255,215,0,0.4)");
            ctx.set_text_align(align);
            ctx.set_fill_style_str("#ffd700");
            ctx.set_font("bold 24px 'Cinzel', serif");
            let text = format!("{} HITS!", combo.hit_count);
            let _ = ctx.fill_text(&text, x, CANVAS_H - 50.0);
            ctx.restore();
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
