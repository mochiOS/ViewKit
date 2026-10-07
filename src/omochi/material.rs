#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurgersParameters {
    pub e1: f32,
    pub e2: f32,
    pub eta1: f32,
    pub eta2: f32,
}

pub const DEFAULT_BURGERS: BurgersParameters = BurgersParameters {
    e1: 3.10,
    e2: 0.92,
    eta1: 14.0,
    eta2: 0.46,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OmochiMaterial {
    pub drag_gain: f32,
    pub max_pull: f32,
    pub press_depth: f32,
    pub press_radius: f32,
    pub drag_radius: f32,
    pub reference_creep_time: f32,
    pub flow_recovery_time: f32,
    pub spring_recovery_time: f32,
    pub tan_delta: f32,
    pub drag_follow: f32,
    pub center_lock: f32,
    pub neck_ratio: f32,
    pub neck_width_ratio: f32,
    pub neck_backshift_ratio: f32,
    pub tip_stretch: f32,
    pub tip_long_radius: f32,
    pub tip_cross_radius: f32,
    pub viscous_follow_time: f32,
    pub fast_viscous_follow_time: f32,
    pub fast_speed: f32,
    pub max_speed: f32,
    pub press_release_start: f32,
    pub press_release_end: f32,
    pub velocity_pull_scale: f32,
    pub velocity_idle_decay: f32,
}
