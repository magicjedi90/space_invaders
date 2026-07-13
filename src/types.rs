use engine_core::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    ModeSelect { selection: u8 },
    Achievements,
    Playing,
    GameOver { won: bool },
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

    pub(crate) player: Option<EntityId>,
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
    pub(crate) player_bullets: Vec<EntityId>,
    pub(crate) invader_bullets: Vec<EntityId>,
    pub(crate) background: Option<EntityId>,
    /// White 1x1 texture for every sprite (the whole game is neon rects).
    pub(crate) tex_id: u32,

    pub(crate) score: u32,
    pub(crate) lives: u32,
    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Horizontal march direction: +1.0 right, -1.0 left.
    pub(crate) march_dir: f32,
    /// Formation displacement from the spawn layout (march + descents).
    pub(crate) formation_offset: Vec2,
    /// Seconds until the player may fire again.
    pub(crate) fire_cooldown: f32,
    /// Kill shots in a row without a wasted bullet (sharpshooter tracking).
    pub(crate) shot_streak: u32,

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
            player: None,
            invaders: Vec::new(),
            ufo: None,
            ufo_dir: 1.0,
            ufo_flash_bonus: 0,
            ufo_flash: 0.0,
            barrier_blocks: Vec::new(),
            player_bullets: Vec::new(),
            invader_bullets: Vec::new(),
            background: None,
            tex_id: 0,
            score: 0,
            lives: crate::constants::STARTING_LIVES,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            march_dir: 1.0,
            formation_offset: Vec2::ZERO,
            fire_cooldown: 0.0,
            shot_streak: 0,
            grid: None,
            debug_colliders: false,
        }
    }
}
