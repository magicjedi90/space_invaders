//! Shooting and hit resolution: the player cannon's trigger discipline,
//! invader return fire, and what happens when bullets meet things.

use engine_core::prelude::*;

use crate::constants::*;
use crate::spawning::invader_value;
use crate::types::*;

use super::{entity_position, rects_overlap};

/// Which live column returns fire, from a pseudo-random draw. `live_cols`
/// must be non-empty; the draw wraps over it.
pub(crate) fn pick_shooter_column(live_cols: &[usize], rand: u32) -> usize {
    live_cols[rand as usize % live_cols.len()]
}

/// The player's trigger discipline for a chaos mode:
/// `(shots_per_fire, max_live_bullets)`.
///
/// Normal keeps the classic one-bullet-in-flight rule. Insane compensates
/// for the faster fleet with more single shots on screen. Ridiculous
/// answers the relentless fire with twin cannons, one volley in flight.
/// Insiculous faces both fleet buffs, so it gets twin cannons AND stacked
/// volleys. An explicit match (not the `is_*` predicates — both are true
/// for Insiculous) so each mode's cap reads directly.
pub(crate) fn player_fire_caps(mode: ChaosMode) -> (usize, usize) {
    match mode {
        ChaosMode::Normal => (1, MAX_PLAYER_BULLETS),
        ChaosMode::Insane => (1, INSANE_MAX_PLAYER_BULLETS),
        ChaosMode::Ridiculous => (2, RIDICULOUS_MAX_PLAYER_BULLETS),
        ChaosMode::Insiculous => (2, INSICULOUS_MAX_PLAYER_BULLETS),
    }
}

/// Average invader shots per second for a chaos mode: Ridiculous (and
/// Insiculous) fleets fire relentlessly; Insane's buff is march speed, not
/// trigger-happiness (see `march_speed`).
pub(crate) fn invader_fire_rate(mode: ChaosMode) -> f32 {
    if mode.is_ridiculous() {
        INVADER_FIRE_RATE * RIDICULOUS_FIRE_MULT
    } else {
        INVADER_FIRE_RATE
    }
}

impl SpaceInvadersGame {
    /// Invader return fire: a per-frame pseudo-random draw against the
    /// fleet's fire rate; the shot comes from the bottom-most invader of a
    /// pseudo-randomly chosen live column.
    pub(crate) fn update_invader_fire(&mut self, ctx: &mut GameContext) {
        if self.invaders.is_empty() {
            return;
        }
        let rate = invader_fire_rate(self.chaos_mode);
        if hash_f32(self.frame_count) >= rate * ctx.delta_time {
            return;
        }

        let mut live_cols: Vec<usize> = self.invaders.iter().map(|i| i.col).collect();
        live_cols.sort_unstable();
        live_cols.dedup();
        let col = pick_shooter_column(&live_cols, hash_u32(self.frame_count));

        // Bottom-most = highest row index in that column.
        let shooter = self.invaders.iter()
            .filter(|i| i.col == col)
            .max_by_key(|i| i.row)
            .map(|i| i.entity);
        if let Some(pos) = shooter.and_then(|e| entity_position(ctx.world, e)) {
            let muzzle = Vec2::new(pos.x, pos.y - INVADER_H / 2.0 - INVADER_BULLET_H / 2.0 - 1.0);
            self.spawn_invader_bullet(ctx.world, muzzle);
        }
    }

    /// Resolve this frame's started collisions: each cannon's bullets kill
    /// invaders and chip barriers; invader bullets hit any live cannon and
    /// chip barriers. Every bullet is spent by its first hit.
    pub(crate) fn resolve_bullet_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);

        // --- Each cannon's own bullets ---
        for index in 0..self.players.len() {
            self.resolve_one_cannons_bullets(ctx, index, collisions, &theme);
        }

        // --- Invader bullets test every LIVE cannon ---
        let mut spent_enemy: Vec<EntityId> = Vec::new();
        let mut cannons_hit: Vec<usize> = Vec::new();
        let enemy_bullets: Vec<EntityId> = self.invader_bullets.clone();
        for bullet in enemy_bullets {
            let hit_index = self.players.iter().position(|p| {
                p.entity.is_some_and(|e| {
                    collisions.iter().any(|c| c.event.started && c.event.involves(bullet, e))
                })
            });
            if let Some(index) = hit_index {
                spent_enemy.push(bullet);
                cannons_hit.push(index);
                continue;
            }
            if self.bullet_chips_barrier(ctx, bullet, collisions, &theme) {
                spent_enemy.push(bullet);
            }
        }
        for bullet in spent_enemy {
            self.invader_bullets.retain(|&b| b != bullet);
            self.physics.destroy_entity(ctx.world, bullet);
        }
        for index in cannons_hit {
            self.player_hit(ctx, index);
        }
    }

    /// Resolve the bullets belonging to cannon `index`: kills score to that
    /// player and extend its streak; barrier chips spend the bullet.
    fn resolve_one_cannons_bullets(
        &mut self,
        ctx: &mut GameContext,
        index: usize,
        collisions: &[CollisionData],
        theme: &ChaosTheme,
    ) {
        let mut spent: Vec<(EntityId, bool)> = Vec::new(); // (bullet, killed an invader)
        let bullets: Vec<EntityId> = self.players[index].bullets.clone();
        for bullet in bullets {
            let hit_invader = self.invaders.iter().position(|i| {
                collisions.iter().any(|c| c.event.started && c.event.involves(bullet, i.entity))
            });
            if let Some(pos_idx) = hit_invader {
                let invader = self.invaders.remove(pos_idx);
                self.players[index].score += invader_value(invader.row);
                if let Some(pos) = entity_position(ctx.world, invader.entity) {
                    let color = INVADER_ROW_COLORS[invader.row.min(INVADER_ROWS - 1)];
                    ctx.particles.spawn_burst(
                        pos, &crate::effects::invader_death_burst(color, theme, self.tex_id));
                    self.ripple_grid(pos, GRID_IMPULSE_KILL_STRENGTH, GRID_IMPULSE_KILL_RADIUS);
                }
                self.physics.destroy_entity(ctx.world, invader.entity);
                spent.push((bullet, true));
                continue;
            }

            if self.bullet_chips_barrier(ctx, bullet, collisions, theme) {
                spent.push((bullet, false));
            }
        }
        for (bullet, killed) in spent {
            self.finish_player_shot(ctx, index, bullet, killed);
        }
    }

    /// Destroy the first barrier block `bullet` reported a contact with.
    /// Returns true if the bullet is spent.
    fn bullet_chips_barrier(
        &mut self,
        ctx: &mut GameContext,
        bullet: EntityId,
        collisions: &[CollisionData],
        theme: &ChaosTheme,
    ) -> bool {
        let hit_block = self.barrier_blocks.iter().copied().find(|&block| {
            collisions.iter().any(|c| c.event.started && c.event.involves(bullet, block))
        });
        let Some(block) = hit_block else { return false };

        if let Some(pos) = entity_position(ctx.world, block) {
            ctx.particles.spawn_burst(
                pos, &crate::effects::barrier_chip_burst(theme.structure_color, theme, self.tex_id));
        }
        self.barrier_blocks.retain(|&b| b != block);
        self.physics.destroy_entity(ctx.world, block);
        true
    }

    /// Bullets flying opposite ways cancel each other in a flash. Both are
    /// dynamic sensors, so rapier would report the pair, but at closing
    /// speeds a whole frame can step past the overlap — a rectangle test on
    /// positions is the reliable arcade answer.
    pub(crate) fn cancel_crossing_bullets(&mut self, ctx: &mut GameContext) {
        if self.invader_bullets.is_empty() {
            return;
        }
        // Inflate by half a frame of closing speed so crossings can't step
        // through the test between frames.
        let closing_pad = (PLAYER_BULLET_SPEED + INVADER_BULLET_SPEED) * ctx.delta_time / 2.0;
        let player_half = Vec2::new(PLAYER_BULLET_W / 2.0, PLAYER_BULLET_H / 2.0 + closing_pad);
        let enemy_half = Vec2::new(INVADER_BULLET_W / 2.0, INVADER_BULLET_H / 2.0);
        let theme = ChaosTheme::for_mode(self.chaos_mode);

        let mut cancelled: Vec<(usize, EntityId, EntityId)> = Vec::new(); // (owner, mine, theirs)
        for index in 0..self.players.len() {
            let bullets: Vec<EntityId> = self.players[index].bullets.clone();
            for mine in bullets {
                let Some(my_pos) = entity_position(ctx.world, mine) else { continue };
                let crossing = self.invader_bullets.iter().copied().find(|&theirs| {
                    cancelled.iter().all(|&(_, _, t)| t != theirs)
                        && entity_position(ctx.world, theirs)
                            .is_some_and(|tp| rects_overlap(my_pos, player_half, tp, enemy_half))
                });
                if let Some(theirs) = crossing {
                    ctx.particles.spawn_burst(
                        my_pos, &crate::effects::bullet_cancel_burst(&theme, self.tex_id));
                    cancelled.push((index, mine, theirs));
                }
            }
        }
        for (index, mine, theirs) in cancelled {
            // A cancel is not a kill: the streak resets.
            self.finish_player_shot(ctx, index, mine, false);
            self.invader_bullets.retain(|&b| b != theirs);
            self.physics.destroy_entity(ctx.world, theirs);
        }
    }

    /// Remove bullets that left the playfield or were despawned by their
    /// `Lifetime` safety net. A player bullet ending this way is a miss.
    pub(crate) fn cull_stray_bullets(&mut self, ctx: &mut GameContext) {
        let top = WIN_H / 2.0 + BULLET_CULL_PAD;
        let bottom = -(WIN_H / 2.0 + BULLET_CULL_PAD);

        for index in 0..self.players.len() {
            let stray: Vec<EntityId> = self.players[index].bullets.iter().copied()
                .filter(|&b| entity_position(ctx.world, b).is_none_or(|p| !p.y.is_finite() || p.y > top))
                .collect();
            for bullet in stray {
                self.finish_player_shot(ctx, index, bullet, false);
            }
        }

        let stray_enemy: Vec<EntityId> = self.invader_bullets.iter().copied()
            .filter(|&b| entity_position(ctx.world, b).is_none_or(|p| !p.y.is_finite() || p.y < bottom))
            .collect();
        for bullet in stray_enemy {
            self.invader_bullets.retain(|&b| b != bullet);
            self.physics.destroy_entity(ctx.world, bullet);
        }
    }

    /// Victory the moment the last invader dies.
    pub(crate) fn check_win_condition(&mut self, ctx: &mut GameContext) {
        if self.state == GameState::Playing && self.invaders.is_empty() {
            self.unlock_win_achievements(ctx);
            self.finish_game(ctx, true);
        }
    }
}
