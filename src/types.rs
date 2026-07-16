use engine_core::prelude::*;

use crate::constants::STARTING_LIVES;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    ModeSelect { selection: u8 },
    Achievements,
    Playing,
    GameOver { won: bool },
}

/// How many cannons defend Earth: one merged cannon, or two co-op cannons
/// sharing the alien formation but keeping individual lives and scores.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GameMode {
    SinglePlayer,
    TwoPlayerCoop,
}

impl GameMode {
    /// Number of cannons this mode fields.
    pub(crate) fn player_count(self) -> usize {
        match self {
            GameMode::SinglePlayer => 1,
            GameMode::TwoPlayerCoop => 2,
        }
    }
}

/// One cannon's per-match state: its body (None once it runs out of lives),
/// its own bullets, cooldown, score, lives, and sharpshooter streak.
pub(crate) struct PlayerState {
    pub(crate) entity: Option<EntityId>,
    pub(crate) bullets: Vec<EntityId>,
    pub(crate) fire_cooldown: f32,
    pub(crate) score: u32,
    pub(crate) lives: u32,
    pub(crate) shot_streak: u32,
}

impl PlayerState {
    /// A cannon at the start of a match: no body yet, full lives.
    pub(crate) fn fresh() -> Self {
        Self {
            entity: None,
            bullets: Vec::new(),
            fire_cooldown: 0.0,
            score: 0,
            lives: STARTING_LIVES,
            shot_streak: 0,
        }
    }
}

/// A live invader: its entity plus the formation slot it marches in.
/// The slot never changes — the whole formation moves as one offset.
pub(crate) struct Invader {
    pub(crate) entity: EntityId,
    pub(crate) row: usize,
    pub(crate) col: usize,
}

pub(crate) struct SpaceInvadersGame {
    pub(crate) physics: PhysicsSystem,
    /// Ticks `Lifetime` components down and despawns expired bullets.
    pub(crate) lifetimes: LifetimeSystem,

    pub(crate) invaders: Vec<Invader>,
    /// The mystery ship crossing the top lane, if one is in flight.
    pub(crate) ufo: Option<EntityId>,
    /// UFO flight direction: +1.0 left-to-right, -1.0 right-to-left.
    pub(crate) ufo_dir: f32,
    /// Bonus paid by the last UFO kill, shown while `ufo_flash` runs.
    pub(crate) ufo_flash_bonus: u32,
    /// Seconds left on the "UFO +N" HUD flash.
    pub(crate) ufo_flash: f32,
    pub(crate) barrier_blocks: Vec<EntityId>,
    /// Invader return fire — shared by the whole fleet, tests every cannon.
    pub(crate) invader_bullets: Vec<EntityId>,
    pub(crate) background: Option<EntityId>,
    /// White 1x1 texture for every sprite (the whole game is neon rects).
    pub(crate) tex_id: u32,

    /// The defending cannons: one in single player, two in co-op. Each keeps
    /// its own lives, score, bullets, and streak.
    pub(crate) players: Vec<PlayerState>,
    /// How many cannons are in play (picked from the title menu).
    pub(crate) mode: GameMode,
    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Horizontal march direction: +1.0 right, -1.0 left.
    pub(crate) march_dir: f32,
    /// Formation displacement from the spawn layout (march + descents).
    pub(crate) formation_offset: Vec2,

    /// Deforming spring-mass grid drawn under the gameplay sprites.
    pub(crate) grid: Option<GridMesh>,
    /// F1 toggles magenta collider outlines over the sprites.
    pub(crate) debug_colliders: bool,
}

impl Default for SpaceInvadersGame {
    fn default() -> Self {
        Self {
            physics: PhysicsSystem::with_config(PhysicsConfig::top_down()),
            lifetimes: LifetimeSystem::new(),
            invaders: Vec::new(),
            ufo: None,
            ufo_dir: 1.0,
            ufo_flash_bonus: 0,
            ufo_flash: 0.0,
            barrier_blocks: Vec::new(),
            invader_bullets: Vec::new(),
            background: None,
            tex_id: 0,
            // One cannon until the title menu picks a mode and `start_game`
            // rebuilds the roster; a lone default player keeps the headless
            // spawn-recipe tests working.
            players: vec![PlayerState::fresh()],
            mode: GameMode::SinglePlayer,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            march_dir: 1.0,
            formation_offset: Vec2::ZERO,
            grid: None,
            debug_colliders: false,
        }
    }
}
