use game_sim::player::PlayerAction;

pub const NUM_JOINTS: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum JointId {
    Hips = 0,
    Torso = 1,
    Neck = 2,
    Head = 3,
    LShoulder = 4,
    RShoulder = 5,
    LElbow = 6,
    RElbow = 7,
    LWrist = 8,
    RWrist = 9,
    LKnee = 10,
    RKnee = 11,
    LAnkle = 12,
    RAnkle = 13,
}

/// Parent joint index for FK chain. Hips is root (parent = self).
const PARENT: [usize; NUM_JOINTS] = [
    0,  // Hips -> Hips (root)
    0,  // Torso -> Hips
    1,  // Neck -> Torso
    2,  // Head -> Neck
    1,  // LShoulder -> Torso
    1,  // RShoulder -> Torso
    4,  // LElbow -> LShoulder
    5,  // RElbow -> RShoulder
    6,  // LWrist -> LElbow
    7,  // RWrist -> RElbow
    0,  // LKnee -> Hips
    0,  // RKnee -> Hips
    10, // LAnkle -> LKnee
    11, // RAnkle -> RKnee
];

#[derive(Debug, Clone, Copy)]
pub struct Joint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct Skeleton {
    pub joints: [Joint; NUM_JOINTS],
}

#[derive(Debug, Clone)]
pub struct Keyframe {
    pub joint_offsets: [(f32, f32); NUM_JOINTS],
    pub duration_frames: u32,
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub keyframes: Vec<Keyframe>,
    pub looping: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimId {
    Idle,
    WalkForward,
    WalkBack,
    Jump,
    Crouch,
    LightAttack1,
    LightAttack2,
    LightAttack3,
    HeavyAttack,
    Uppercut,
    AerialAttack,
    Block,
    Fireball,
    DashStrike,
    Hitstun,
    Blockstun,
    Knockdown,
    Getup,
}

impl AnimId {
    pub fn from_action(action: &PlayerAction) -> Self {
        match action {
            PlayerAction::Idle => AnimId::Idle,
            PlayerAction::WalkForward => AnimId::WalkForward,
            PlayerAction::WalkBack => AnimId::WalkBack,
            PlayerAction::Jump => AnimId::Jump,
            PlayerAction::Crouch => AnimId::Crouch,
            PlayerAction::LightAttack1 => AnimId::LightAttack1,
            PlayerAction::LightAttack2 => AnimId::LightAttack2,
            PlayerAction::LightAttack3 => AnimId::LightAttack3,
            PlayerAction::HeavyAttack => AnimId::HeavyAttack,
            PlayerAction::Uppercut => AnimId::Uppercut,
            PlayerAction::AerialAttack => AnimId::AerialAttack,
            PlayerAction::Block => AnimId::Block,
            PlayerAction::Fireball => AnimId::Fireball,
            PlayerAction::DashStrike => AnimId::DashStrike,
            PlayerAction::Hitstun { .. } => AnimId::Hitstun,
            PlayerAction::Blockstun { .. } => AnimId::Blockstun,
            PlayerAction::Knockdown { .. } => AnimId::Knockdown,
            PlayerAction::Getup => AnimId::Getup,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnimationState {
    pub anim_id: AnimId,
    pub frame: f32,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationState {
    pub fn new() -> Self {
        AnimationState {
            anim_id: AnimId::Idle,
            frame: 0.0,
        }
    }

    pub fn set(&mut self, id: AnimId) {
        if self.anim_id != id {
            self.anim_id = id;
            self.frame = 0.0;
        }
    }

    pub fn advance(&mut self, anim: &Animation) {
        self.frame += 1.0;
        let total: f32 = anim.keyframes.iter().map(|k| k.duration_frames as f32).sum();
        if total <= 0.0 {
            return;
        }
        if self.frame >= total {
            if anim.looping {
                self.frame %= total;
            } else {
                self.frame = total - 0.01;
            }
        }
    }
}

/// Default pose offsets — a standing humanoid figure.
/// Coordinates are (x_offset_from_parent, y_offset_from_parent).
/// Y-axis points down (screen space). Character ~80px tall.
fn base_pose() -> [(f32, f32); NUM_JOINTS] {
    [
        (0.0, 0.0),     // Hips (root)
        (0.0, -25.0),   // Torso
        (0.0, -20.0),   // Neck
        (0.0, -10.0),   // Head
        (-12.0, -2.0),  // LShoulder
        (12.0, -2.0),   // RShoulder
        (-10.0, 15.0),  // LElbow
        (10.0, 15.0),   // RElbow
        (-3.0, 14.0),   // LWrist
        (3.0, 14.0),    // RWrist
        (-6.0, 20.0),   // LKnee
        (6.0, 20.0),    // RKnee
        (0.0, 22.0),    // LAnkle
        (0.0, 22.0),    // RAnkle
    ]
}

fn make_keyframe(offsets: [(f32, f32); NUM_JOINTS], dur: u32) -> Keyframe {
    Keyframe {
        joint_offsets: offsets,
        duration_frames: dur,
    }
}

fn offset_joints(base: &[(f32, f32); NUM_JOINTS], changes: &[(usize, f32, f32)]) -> [(f32, f32); NUM_JOINTS] {
    let mut out = *base;
    for &(idx, dx, dy) in changes {
        out[idx].0 += dx;
        out[idx].1 += dy;
    }
    out
}

pub fn get_animation(id: AnimId) -> Animation {
    let bp = base_pose();
    match id {
        AnimId::Idle => {
            let up = offset_joints(&bp, &[
                (JointId::Torso as usize, 0.0, -1.5),
                (JointId::Head as usize, 0.0, -0.5),
            ]);
            let down = bp;
            Animation {
                keyframes: vec![make_keyframe(up, 30), make_keyframe(down, 30)],
                looping: true,
            }
        }
        AnimId::WalkForward => {
            let step1 = offset_joints(&bp, &[
                (JointId::LKnee as usize, 3.0, -5.0),
                (JointId::LAnkle as usize, 2.0, -3.0),
                (JointId::RKnee as usize, -2.0, 2.0),
                (JointId::LShoulder as usize, 2.0, 0.0),
                (JointId::RShoulder as usize, -2.0, 0.0),
            ]);
            let step2 = offset_joints(&bp, &[
                (JointId::RKnee as usize, 3.0, -5.0),
                (JointId::RAnkle as usize, 2.0, -3.0),
                (JointId::LKnee as usize, -2.0, 2.0),
                (JointId::RShoulder as usize, 2.0, 0.0),
                (JointId::LShoulder as usize, -2.0, 0.0),
            ]);
            Animation {
                keyframes: vec![make_keyframe(step1, 10), make_keyframe(step2, 10)],
                looping: true,
            }
        }
        AnimId::WalkBack => {
            let step1 = offset_joints(&bp, &[
                (JointId::LKnee as usize, -3.0, -5.0),
                (JointId::LAnkle as usize, -2.0, -3.0),
                (JointId::RKnee as usize, 2.0, 2.0),
            ]);
            let step2 = offset_joints(&bp, &[
                (JointId::RKnee as usize, -3.0, -5.0),
                (JointId::RAnkle as usize, -2.0, -3.0),
                (JointId::LKnee as usize, 2.0, 2.0),
            ]);
            Animation {
                keyframes: vec![make_keyframe(step1, 12), make_keyframe(step2, 12)],
                looping: true,
            }
        }
        AnimId::Jump => {
            let crouch = offset_joints(&bp, &[
                (JointId::Hips as usize, 0.0, 5.0),
                (JointId::LKnee as usize, -3.0, -8.0),
                (JointId::RKnee as usize, 3.0, -8.0),
            ]);
            let air = offset_joints(&bp, &[
                (JointId::Hips as usize, 0.0, -5.0),
                (JointId::LKnee as usize, -3.0, 5.0),
                (JointId::RKnee as usize, 3.0, 5.0),
                (JointId::LWrist as usize, -5.0, -5.0),
                (JointId::RWrist as usize, 5.0, -5.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(crouch, 3),
                    make_keyframe(air, 20),
                    make_keyframe(bp, 3),
                ],
                looping: false,
            }
        }
        AnimId::Crouch => {
            let crouched = offset_joints(&bp, &[
                (JointId::Hips as usize, 0.0, 15.0),
                (JointId::Torso as usize, 0.0, -5.0),
                (JointId::LKnee as usize, -5.0, -10.0),
                (JointId::RKnee as usize, 5.0, -10.0),
                (JointId::LAnkle as usize, -3.0, -5.0),
                (JointId::RAnkle as usize, 3.0, -5.0),
            ]);
            Animation {
                keyframes: vec![make_keyframe(crouched, 60)],
                looping: true,
            }
        }
        AnimId::LightAttack1 => {
            let startup = offset_joints(&bp, &[
                (JointId::RElbow as usize, 5.0, -8.0),
                (JointId::RWrist as usize, 8.0, -10.0),
            ]);
            let active = offset_joints(&bp, &[
                (JointId::RElbow as usize, 15.0, -3.0),
                (JointId::RWrist as usize, 25.0, -2.0),
                (JointId::Torso as usize, 3.0, 0.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(startup, 3),
                    make_keyframe(active, 2),
                    make_keyframe(bp, 4),
                ],
                looping: false,
            }
        }
        AnimId::LightAttack2 => {
            let startup = offset_joints(&bp, &[
                (JointId::LElbow as usize, -5.0, -8.0),
                (JointId::LWrist as usize, -8.0, -10.0),
            ]);
            let active = offset_joints(&bp, &[
                (JointId::LElbow as usize, -15.0, -3.0),
                (JointId::LWrist as usize, -25.0, -2.0),
                (JointId::Torso as usize, -3.0, 0.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(startup, 2),
                    make_keyframe(active, 2),
                    make_keyframe(bp, 4),
                ],
                looping: false,
            }
        }
        AnimId::LightAttack3 => {
            let startup = offset_joints(&bp, &[
                (JointId::RElbow as usize, 8.0, -10.0),
                (JointId::RWrist as usize, 15.0, -12.0),
                (JointId::Torso as usize, 2.0, -2.0),
            ]);
            let active = offset_joints(&bp, &[
                (JointId::RElbow as usize, 20.0, 0.0),
                (JointId::RWrist as usize, 32.0, 2.0),
                (JointId::Torso as usize, 5.0, 0.0),
                (JointId::LKnee as usize, 3.0, -3.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(startup, 3),
                    make_keyframe(active, 2),
                    make_keyframe(bp, 5),
                ],
                looping: false,
            }
        }
        AnimId::HeavyAttack => {
            let windup = offset_joints(&bp, &[
                (JointId::RElbow as usize, -5.0, -12.0),
                (JointId::RWrist as usize, -8.0, -18.0),
                (JointId::Torso as usize, -3.0, -2.0),
            ]);
            let active = offset_joints(&bp, &[
                (JointId::RElbow as usize, 18.0, 5.0),
                (JointId::RWrist as usize, 30.0, 10.0),
                (JointId::Torso as usize, 5.0, 2.0),
                (JointId::Hips as usize, 3.0, 0.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(windup, 5),
                    make_keyframe(active, 3),
                    make_keyframe(bp, 6),
                ],
                looping: false,
            }
        }
        AnimId::Uppercut => {
            let crouch = offset_joints(&bp, &[
                (JointId::Hips as usize, 0.0, 5.0),
                (JointId::LKnee as usize, -4.0, -5.0),
                (JointId::RKnee as usize, 4.0, -5.0),
                (JointId::RElbow as usize, 3.0, 5.0),
                (JointId::RWrist as usize, 5.0, 8.0),
            ]);
            let strike = offset_joints(&bp, &[
                (JointId::Hips as usize, 0.0, -5.0),
                (JointId::RElbow as usize, 5.0, -20.0),
                (JointId::RWrist as usize, 8.0, -30.0),
                (JointId::Torso as usize, 0.0, -5.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(crouch, 3),
                    make_keyframe(strike, 2),
                    make_keyframe(bp, 5),
                ],
                looping: false,
            }
        }
        AnimId::AerialAttack => {
            let windup = offset_joints(&bp, &[
                (JointId::RElbow as usize, 10.0, -5.0),
                (JointId::RWrist as usize, 15.0, -8.0),
                (JointId::LKnee as usize, 0.0, 5.0),
                (JointId::RKnee as usize, 0.0, 5.0),
            ]);
            let slash = offset_joints(&bp, &[
                (JointId::RElbow as usize, 15.0, 10.0),
                (JointId::RWrist as usize, 25.0, 15.0),
                (JointId::Torso as usize, 3.0, 3.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(windup, 2),
                    make_keyframe(slash, 2),
                    make_keyframe(bp, 4),
                ],
                looping: false,
            }
        }
        AnimId::Fireball => {
            let windup = offset_joints(&bp, &[
                (JointId::RElbow as usize, 5.0, -10.0),
                (JointId::RWrist as usize, 10.0, -15.0),
                (JointId::LElbow as usize, -3.0, -5.0),
                (JointId::Torso as usize, -2.0, -1.0),
            ]);
            let cast = offset_joints(&bp, &[
                (JointId::RElbow as usize, 18.0, -5.0),
                (JointId::RWrist as usize, 28.0, -3.0),
                (JointId::LElbow as usize, -8.0, -3.0),
                (JointId::LWrist as usize, -12.0, -5.0),
                (JointId::Torso as usize, 4.0, 0.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(windup, 6),
                    make_keyframe(cast, 4),
                    make_keyframe(bp, 6),
                ],
                looping: false,
            }
        }
        AnimId::DashStrike => {
            let crouch = offset_joints(&bp, &[
                (JointId::Hips as usize, 0.0, 5.0),
                (JointId::Torso as usize, 5.0, -2.0),
                (JointId::LKnee as usize, -4.0, -5.0),
                (JointId::RKnee as usize, 4.0, -5.0),
                (JointId::RElbow as usize, -5.0, -5.0),
                (JointId::RWrist as usize, -8.0, -8.0),
            ]);
            let dash = offset_joints(&bp, &[
                (JointId::Torso as usize, 10.0, -3.0),
                (JointId::Hips as usize, 5.0, 0.0),
                (JointId::RElbow as usize, 20.0, 0.0),
                (JointId::RWrist as usize, 30.0, 2.0),
                (JointId::LKnee as usize, -5.0, 3.0),
                (JointId::RKnee as usize, 8.0, -3.0),
            ]);
            Animation {
                keyframes: vec![
                    make_keyframe(crouch, 4),
                    make_keyframe(dash, 4),
                    make_keyframe(bp, 4),
                ],
                looping: false,
            }
        }
        AnimId::Block => {
            let guard = offset_joints(&bp, &[
                (JointId::LElbow as usize, 5.0, -10.0),
                (JointId::LWrist as usize, 8.0, -15.0),
                (JointId::RElbow as usize, -3.0, -8.0),
                (JointId::RWrist as usize, -5.0, -12.0),
                (JointId::Hips as usize, 0.0, 2.0),
                (JointId::LKnee as usize, -2.0, -3.0),
                (JointId::RKnee as usize, 2.0, -3.0),
            ]);
            Animation {
                keyframes: vec![make_keyframe(guard, 60)],
                looping: true,
            }
        }
        AnimId::Hitstun => {
            let recoil = offset_joints(&bp, &[
                (JointId::Torso as usize, -5.0, 2.0),
                (JointId::Head as usize, -3.0, 2.0),
                (JointId::Hips as usize, -3.0, 2.0),
            ]);
            Animation {
                keyframes: vec![make_keyframe(recoil, 4), make_keyframe(bp, 6)],
                looping: false,
            }
        }
        AnimId::Blockstun => {
            let push = offset_joints(&bp, &[
                (JointId::LElbow as usize, 5.0, -10.0),
                (JointId::LWrist as usize, 8.0, -15.0),
                (JointId::Hips as usize, -4.0, 2.0),
                (JointId::Torso as usize, -3.0, 1.0),
            ]);
            Animation {
                keyframes: vec![make_keyframe(push, 3), make_keyframe(bp, 5)],
                looping: false,
            }
        }
        AnimId::Knockdown => {
            let fall = offset_joints(&bp, &[
                (JointId::Hips as usize, 0.0, 25.0),
                (JointId::Torso as usize, -15.0, 10.0),
                (JointId::Head as usize, -5.0, 5.0),
                (JointId::LKnee as usize, 5.0, -10.0),
                (JointId::RKnee as usize, -5.0, -10.0),
                (JointId::LAnkle as usize, 3.0, -8.0),
                (JointId::RAnkle as usize, -3.0, -8.0),
                (JointId::LWrist as usize, -8.0, 5.0),
                (JointId::RWrist as usize, 8.0, 5.0),
            ]);
            Animation {
                keyframes: vec![make_keyframe(fall, 15)],
                looping: false,
            }
        }
        AnimId::Getup => {
            let crouched = offset_joints(&bp, &[
                (JointId::Hips as usize, 0.0, 15.0),
                (JointId::Torso as usize, -5.0, 5.0),
                (JointId::LKnee as usize, -5.0, -8.0),
                (JointId::RKnee as usize, 5.0, -8.0),
            ]);
            Animation {
                keyframes: vec![make_keyframe(crouched, 8), make_keyframe(bp, 7)],
                looping: false,
            }
        }
    }
}

/// Forward kinematics: compute world-space joint positions.
pub fn compute_skeleton(root_x: f32, root_y: f32, facing: f32, anim: &Animation, frame: f32) -> Skeleton {
    let offsets = interpolate_keyframe(anim, frame);

    let mut joints = [Joint { x: 0.0, y: 0.0 }; NUM_JOINTS];

    // Root (Hips)
    joints[0] = Joint {
        x: root_x + offsets[0].0 * facing,
        y: root_y + offsets[0].1,
    };

    // FK chain: process children in order (PARENT[i] < i for all i > 0)
    for i in 1..NUM_JOINTS {
        let parent = PARENT[i];
        joints[i] = Joint {
            x: joints[parent].x + offsets[i].0 * facing,
            y: joints[parent].y + offsets[i].1,
        };
    }

    Skeleton { joints }
}

/// Interpolate between keyframes based on fractional frame position.
fn interpolate_keyframe(anim: &Animation, frame: f32) -> [(f32, f32); NUM_JOINTS] {
    if anim.keyframes.is_empty() {
        return [(0.0, 0.0); NUM_JOINTS];
    }
    if anim.keyframes.len() == 1 {
        return anim.keyframes[0].joint_offsets;
    }

    let mut accumulated = 0.0_f32;
    for (i, kf) in anim.keyframes.iter().enumerate() {
        let dur = kf.duration_frames as f32;
        if frame < accumulated + dur {
            let t = (frame - accumulated) / dur;
            let next_i = if i + 1 < anim.keyframes.len() {
                i + 1
            } else if anim.looping {
                0
            } else {
                i
            };
            return lerp_offsets(&anim.keyframes[i].joint_offsets, &anim.keyframes[next_i].joint_offsets, t);
        }
        accumulated += dur;
    }

    anim.keyframes.last().unwrap().joint_offsets
}

fn lerp_offsets(a: &[(f32, f32); NUM_JOINTS], b: &[(f32, f32); NUM_JOINTS], t: f32) -> [(f32, f32); NUM_JOINTS] {
    let mut out = [(0.0_f32, 0.0_f32); NUM_JOINTS];
    for i in 0..NUM_JOINTS {
        out[i].0 = a[i].0 + (b[i].0 - a[i].0) * t;
        out[i].1 = a[i].1 + (b[i].1 - a[i].1) * t;
    }
    out
}
