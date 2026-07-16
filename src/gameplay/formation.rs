//! Formation marching: the fleet moves as one shared offset applied to
//! every invader's home slot. March right until the rightmost live invader
//! touches the bound, then descend one step and reverse — the classic
//! accelerating heartbeat as the fleet thins out.

use engine_core::prelude::*;

use crate::constants::*;
use crate::spawning::{invader_home_x, invader_home_y};
use crate::types::*;

use super::{entity_position, rects_overlap};

/// Horizontal march speed for the current fleet strength: ramps linearly
/// from `MARCH_SPEED_BASE` (full fleet) to `MARCH_SPEED_MAX` (last invader).
/// Insane mode multiplies the result.
pub(crate) fn march_speed(alive: usize, total: usize, insane: bool) -> f32 {
    let total = total.max(1);
    let alive = alive.clamp(1, total);
    // 0.0 with the fleet intact → 1.0 when one invader remains.
    let thinned = (total - alive) as f32 / total.max(2).saturating_sub(1) as f32;
    let speed = MARCH_SPEED_BASE + (MARCH_SPEED_MAX - MARCH_SPEED_BASE) * thinned;
    if insane { speed * INSANE_MARCH_MULT } else { speed }
}

/// One march tick's outcome: the new horizontal offset and direction, and
/// whether the fleet hit a bound and must descend one step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MarchOutcome {
    pub(crate) offset_x: f32,
    pub(crate) dir: f32,
    pub(crate) descended: bool,
}

/// Advance the formation offset by `dx` in direction `dir`, bouncing off
/// `MARCH_BOUND_X`. `live_min_home_x` / `live_max_home_x` are the home
/// center X of the outermost LIVE invaders — dead columns don't shrink the
/// playfield, they widen the march span.
pub(crate) fn march_step(
    offset_x: f32,
    dir: f32,
    dx: f32,
    live_min_home_x: f32,
    live_max_home_x: f32,
) -> MarchOutcome {
    let new_x = offset_x + dir * dx;
    if dir > 0.0 && live_max_home_x + new_x >= MARCH_BOUND_X {
        MarchOutcome { offset_x: MARCH_BOUND_X - live_max_home_x, dir: -1.0, descended: true }
    } else if dir < 0.0 && live_min_home_x + new_x <= -MARCH_BOUND_X {
        MarchOutcome { offset_x: -MARCH_BOUND_X - live_min_home_x, dir: 1.0, descended: true }
    } else {
        MarchOutcome { offset_x: new_x, dir, descended: false }
    }
}

impl SpaceInvadersGame {
    /// March the fleet and retarget every invader's kinematic body to its
    /// home slot plus the shared offset. Low invaders chew through barrier
    /// blocks they overlap.
    pub(crate) fn march_formation(&mut self, ctx: &mut GameContext) {
        if self.invaders.is_empty() {
            return;
        }

        let live_min = self.invaders.iter().map(|i| i.col).min().map(invader_home_x).unwrap_or(0.0);
        let live_max = self.invaders.iter().map(|i| i.col).max().map(invader_home_x).unwrap_or(0.0);

        let speed = march_speed(
            self.invaders.len(),
            INVADER_ROWS * INVADER_COLS,
            self.chaos_mode.is_insane(),
        );
        let outcome = march_step(
            self.formation_offset.x, self.march_dir, speed * ctx.delta_time, live_min, live_max);
        self.formation_offset.x = outcome.offset_x;
        self.march_dir = outcome.dir;
        if outcome.descended {
            self.formation_offset.y -= DESCEND_STEP;
        }

        for invader in &self.invaders {
            let home = Vec2::new(invader_home_x(invader.col), invader_home_y(invader.row));
            self.physics.set_kinematic_target(invader.entity, home + self.formation_offset, 0.0);
        }

        self.chomp_barriers(ctx);
    }

    /// Marching invaders destroy any barrier block they overlap. Rapier
    /// never reports kinematic-vs-static pairs, so this is a game-side
    /// rectangle test — skipped entirely until the fleet descends into
    /// barrier altitude.
    fn chomp_barriers(&mut self, ctx: &mut GameContext) {
        let barrier_top = BARRIER_Y
            + (BARRIER_BLOCK_ROWS as f32 * BARRIER_BLOCK) / 2.0
            + INVADER_H;
        let low_invaders: Vec<Vec2> = self.invaders.iter()
            .filter_map(|i| entity_position(ctx.world, i.entity))
            .filter(|p| p.y < barrier_top)
            .collect();
        if low_invaders.is_empty() {
            return;
        }

        let invader_half = Vec2::new(INVADER_W / 2.0, INVADER_H / 2.0);
        let block_half = Vec2::splat(BARRIER_BLOCK / 2.0);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let mut chomped: Vec<EntityId> = Vec::new();
        self.barrier_blocks.retain(|&block| {
            let Some(pos) = entity_position(ctx.world, block) else { return false };
            let hit = low_invaders.iter()
                .any(|&inv| rects_overlap(inv, invader_half, pos, block_half));
            if hit {
                ctx.particles.spawn_burst(
                    pos, &crate::effects::barrier_chip_burst(theme.structure_color, &theme, self.tex_id));
                chomped.push(block);
            }
            !hit
        });
        for block in chomped {
            self.physics.destroy_entity(ctx.world, block);
        }
    }

    /// The fleet lands when any invader reaches the invasion line or touches
    /// a LIVE cannon — immediate defeat, regardless of remaining lives. A
    /// despawned cannon (a player out of lives) is not a target.
    pub(crate) fn check_invasion(&mut self, ctx: &mut GameContext) {
        if self.state != GameState::Playing {
            return;
        }
        let cannons: Vec<Vec2> = self.players.iter()
            .filter_map(|p| p.entity)
            .filter_map(|e| entity_position(ctx.world, e))
            .collect();
        let invaders: Vec<Vec2> = self.invaders.iter()
            .filter_map(|i| entity_position(ctx.world, i.entity))
            .collect();
        if fleet_has_landed(&invaders, &cannons) {
            self.finish_game(ctx, false);
        }
    }
}

/// Has the fleet landed? True when any invader has crossed the invasion line
/// or overlaps any of the given (live) cannon positions. Dead cannons are
/// simply absent from `cannons`, so they never register as a touch.
pub(crate) fn fleet_has_landed(invaders: &[Vec2], cannons: &[Vec2]) -> bool {
    let invader_half = Vec2::new(INVADER_W / 2.0, INVADER_H / 2.0);
    let player_half = Vec2::new(PLAYER_W / 2.0, PLAYER_H / 2.0);
    invaders.iter().any(|&pos| {
        pos.y <= INVASION_Y
            || cannons.iter().any(|&pp| rects_overlap(pos, invader_half, pp, player_half))
    })
}
