use engine_core::prelude::*;

pub(crate) const WIN_W: f32 = 800.0;
pub(crate) const WIN_H: f32 = 600.0;

// --- Player cannon ---
pub(crate) const PLAYER_W: f32 = 52.0;
pub(crate) const PLAYER_H: f32 = 18.0;
pub(crate) const PLAYER_SCALE: Vec2 = Vec2::new(PLAYER_W / RENDER_UNIT, PLAYER_H / RENDER_UNIT);
pub(crate) const PLAYER_Y: f32 = -250.0;
pub(crate) const PLAYER_SPEED: f32 = 420.0;
pub(crate) const PLAYER_MAX_X: f32 = WIN_W / 2.0 - 20.0 - PLAYER_W / 2.0;

// --- Bullets ---
pub(crate) const PLAYER_BULLET_W: f32 = 5.0;
pub(crate) const PLAYER_BULLET_H: f32 = 14.0;
pub(crate) const PLAYER_BULLET_SPEED: f32 = 540.0;
pub(crate) const INVADER_BULLET_W: f32 = 6.0;
pub(crate) const INVADER_BULLET_H: f32 = 12.0;
pub(crate) const INVADER_BULLET_SPEED: f32 = 240.0;
/// Safety-net auto-despawn for any bullet that somehow escapes the
/// off-screen culling (engine `Lifetime` component).
pub(crate) const BULLET_LIFETIME: f32 = 2.5;
/// Seconds between player shots.
pub(crate) const FIRE_COOLDOWN: f32 = 0.4;
/// Live player bullets allowed at once (classic single-shot discipline).
pub(crate) const MAX_PLAYER_BULLETS: usize = 1;
/// Insane mode: live-bullet cap — extra shots on screen compensate for the
/// faster, angrier fleet.
pub(crate) const INSANE_MAX_PLAYER_BULLETS: usize = 3;
/// Ridiculous mode: live-bullet cap for the twin cannon (3 volleys of 2).
pub(crate) const RIDICULOUS_MAX_PLAYER_BULLETS: usize = 6;
/// Ridiculous mode: horizontal offset of each twin-cannon barrel.
pub(crate) const TWIN_CANNON_OFFSET: f32 = 12.0;
pub(crate) const INVADER_BULLET_COLOR: Vec4 = Vec4::new(1.0, 0.4, 0.3, 1.0);

// --- Invader formation ---
pub(crate) const INVADER_COLS: usize = 10;
pub(crate) const INVADER_ROWS: usize = 5;
pub(crate) const INVADER_W: f32 = 36.0;
pub(crate) const INVADER_H: f32 = 24.0;
pub(crate) const INVADER_SCALE: Vec2 = Vec2::new(INVADER_W / RENDER_UNIT, INVADER_H / RENDER_UNIT);
pub(crate) const INVADER_GAP_X: f32 = 16.0;
pub(crate) const INVADER_GAP_Y: f32 = 18.0;
/// Y position of the center of the top invader row at spawn.
pub(crate) const FORMATION_TOP_Y: f32 = 230.0;
/// The formation's leftmost/rightmost invader center never marches past ±this.
pub(crate) const MARCH_BOUND_X: f32 = WIN_W / 2.0 - 30.0;
/// Horizontal march speed with the formation intact.
pub(crate) const MARCH_SPEED_BASE: f32 = 30.0;
/// Horizontal march speed as the last invader standing.
pub(crate) const MARCH_SPEED_MAX: f32 = 240.0;
/// Insane mode: march speed multiplier.
pub(crate) const INSANE_MARCH_MULT: f32 = 1.8;
/// Vertical drop on every edge bounce.
pub(crate) const DESCEND_STEP: f32 = 24.0;
/// An invader center at or below this line means the fleet has landed.
pub(crate) const INVASION_Y: f32 = -215.0;

/// Average invader shots per second across the whole fleet.
pub(crate) const INVADER_FIRE_RATE: f32 = 0.7;
/// Insane mode: fire-rate multiplier.
pub(crate) const INSANE_FIRE_MULT: f32 = 2.4;

/// Points per kill, by row (0 = top row, scores the most — classic table).
pub(crate) const INVADER_ROW_VALUES: [u32; INVADER_ROWS] = [30, 20, 20, 10, 10];
/// Row tints, top to bottom.
pub(crate) const INVADER_ROW_COLORS: [Vec4; INVADER_ROWS] = [
    Vec4::new(1.0, 0.35, 0.75, 1.0), // magenta
    Vec4::new(1.0, 0.55, 0.25, 1.0), // orange
    Vec4::new(1.0, 0.9, 0.3, 1.0),   // yellow
    Vec4::new(0.35, 0.95, 0.5, 1.0), // green
    Vec4::new(0.35, 0.7, 1.0, 1.0),  // blue
];

// --- Barriers (bunkers) ---
pub(crate) const BARRIER_COUNT: usize = 4;
/// Each barrier is a grid of small destructible blocks.
pub(crate) const BARRIER_BLOCK: f32 = 12.0;
pub(crate) const BARRIER_BLOCK_COLS: usize = 6;
pub(crate) const BARRIER_BLOCK_ROWS: usize = 3;
/// Y position of a barrier's center row.
pub(crate) const BARRIER_Y: f32 = -180.0;
/// X positions of the barrier centers.
pub(crate) const BARRIER_XS: [f32; BARRIER_COUNT] = [-270.0, -90.0, 90.0, 270.0];

pub(crate) const STARTING_LIVES: u32 = 3;
/// Consecutive kill shots (no misses in between) for the streak achievement.
pub(crate) const SHARPSHOOTER_TARGET: u32 = 10;

/// Extra margin past the window edge before a bullet is culled.
pub(crate) const BULLET_CULL_PAD: f32 = 30.0;

pub(crate) const PLAYER_EMISSIVE: f32 = 1.5;
pub(crate) const BULLET_EMISSIVE: f32 = 2.5;
pub(crate) const INVADER_EMISSIVE: f32 = 0.9;
pub(crate) const BARRIER_EMISSIVE: f32 = 0.6;

// Radial impulses kicked into the spring-mass background grid.
pub(crate) const GRID_IMPULSE_KILL_STRENGTH: f32 = 260.0;
pub(crate) const GRID_IMPULSE_KILL_RADIUS: f32 = 90.0;
pub(crate) const GRID_IMPULSE_PLAYER_HIT_STRENGTH: f32 = 700.0;
pub(crate) const GRID_IMPULSE_PLAYER_HIT_RADIUS: f32 = 160.0;
