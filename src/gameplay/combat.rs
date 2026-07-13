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
    /// Hold-to-fire with a cooldown. The chaos mode sets the trigger
    /// discipline — see [`player_fire_caps`].
    pub(crate) fn update_firing(&mut self, ctx: &mut GameContext) {
        self.fire_cooldown = (self.fire_cooldown - ctx.delta_time).max(0.0);

        let trigger = ctx.input.is_key_pressed(KeyCode::Space)
            || ctx.input.is_mouse_button_pressed(MouseButton::Left);
        if !trigger || self.fire_cooldown > 0.0 {
            return;
        }

        let (shots, max_live) = player_fire_caps(self.chaos_mode);
        if self.player_bullets.len() + shots > max_live {
            return;
        }
        let Some(pos) = self.player.and_then(|p| entity_position(ctx.world, p)) else { return };

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let muzzle_y = pos.y + PLAYER_H / 2.0 + PLAYER_BULLET_H / 2.0 + 1.0;
        if shots == 2 {
            for dx in [-TWIN_CANNON_OFFSET, TWIN_CANNON_OFFSET] {
                self.spawn_player_bullet(
                    ctx.world, Vec2::new(pos.x + dx, muzzle_y), theme.accent_color);
            }
        } else {
            self.spawn_player_bullet(ctx.world, Vec2::new(pos.x, muzzle_y), theme.accent_color);
        }
        self.fire_cooldown = FIRE_COOLDOWN;
    }

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

    /// Resolve this frame's started collisions: player bullets kill
    /// invaders and chip barriers; invader bullets hit the cannon and chip
    /// barriers. Every bullet is spent by its first hit.
    pub(crate) fn resolve_bullet_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);

        // --- Player bullets ---
        let mut spent: Vec<(EntityId, bool)> = Vec::new(); // (bullet, killed an invader)
        let bullets: Vec<EntityId> = self.player_bullets.clone();
        for bullet in bullets {
            let hit_invader = self.invaders.iter().position(|i| {
                collisions.iter().any(|c| c.event.started && c.event.involves(bullet, i.entity))
            });
            if let Some(index) = hit_invader {
                let invader = self.invaders.remove(index);
                self.score += invader_value(invader.row);
                if let Some(pos) = entity_position(ctx.world, invader.entity) {
                    let color = INVADER_ROW_COLORS[invader.row.min(INVADER_ROWS - 1)];
                    ctx.particles.spawn_burst(
                        pos, &crate::effects::invader_death_burst(color, &theme, self.tex_id));
                    self.ripple_grid(pos, GRID_IMPULSE_KILL_STRENGTH, GRID_IMPULSE_KILL_RADIUS);
                }
                self.physics.destroy_entity(ctx.world, invader.entity);
                spent.push((bullet, true));
                continue;
            }

            if self.bullet_chips_barrier(ctx, bullet, collisions, &theme) {
                spent.push((bullet, false));
            }
        }
        for (bullet, killed) in spent {
            self.finish_player_shot(ctx, bullet, killed);
        }

        // --- Invader bullets ---
        let player = self.player;
        let mut player_was_hit = false;
        let mut spent_enemy: Vec<EntityId> = Vec::new();
        let enemy_bullets: Vec<EntityId> = self.invader_bullets.clone();
        for bullet in enemy_bullets {
            let hit_player = player.is_some_and(|p| {
                collisions.iter().any(|c| c.event.started && c.event.involves(bullet, p))
            });
            if hit_player {
                spent_enemy.push(bullet);
                player_was_hit = true;
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
        if player_was_hit {
            self.player_hit(ctx);
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
        if self.player_bullets.is_empty() || self.invader_bullets.is_empty() {
            return;
        }
        // Inflate by half a frame of closing speed so crossings can't step
        // through the test between frames.
        let closing_pad = (PLAYER_BULLET_SPEED + INVADER_BULLET_SPEED) * ctx.delta_time / 2.0;
        let player_half = Vec2::new(PLAYER_BULLET_W / 2.0, PLAYER_BULLET_H / 2.0 + closing_pad);
        let enemy_half = Vec2::new(INVADER_BULLET_W / 2.0, INVADER_BULLET_H / 2.0);
        let theme = ChaosTheme::for_mode(self.chaos_mode);

        let mut cancelled: Vec<(EntityId, EntityId)> = Vec::new();
        for &mine in &self.player_bullets {
            let Some(my_pos) = entity_position(ctx.world, mine) else { continue };
            let crossing = self.invader_bullets.iter().copied().find(|&theirs| {
                cancelled.iter().all(|&(_, t)| t != theirs)
                    && entity_position(ctx.world, theirs)
                        .is_some_and(|tp| rects_overlap(my_pos, player_half, tp, enemy_half))
            });
            if let Some(theirs) = crossing {
                ctx.particles.spawn_burst(
                    my_pos, &crate::effects::bullet_cancel_burst(&theme, self.tex_id));
                cancelled.push((mine, theirs));
            }
        }
        for (mine, theirs) in cancelled {
            // A cancel is not a kill: the streak resets.
            self.finish_player_shot(ctx, mine, false);
            self.invader_bullets.retain(|&b| b != theirs);
            self.physics.destroy_entity(ctx.world, theirs);
        }
    }

    /// Remove bullets that left the playfield or were despawned by their
    /// `Lifetime` safety net. A player bullet ending this way is a miss.
    pub(crate) fn cull_stray_bullets(&mut self, ctx: &mut GameContext) {
        let top = WIN_H / 2.0 + BULLET_CULL_PAD;
        let bottom = -(WIN_H / 2.0 + BULLET_CULL_PAD);

        let stray: Vec<EntityId> = self.player_bullets.iter().copied()
            .filter(|&b| entity_position(ctx.world, b).is_none_or(|p| !p.y.is_finite() || p.y > top))
            .collect();
        for bullet in stray {
            self.finish_player_shot(ctx, bullet, false);
        }

        let stray_enemy: Vec<EntityId> = self.invader_bullets.iter().copied()
            .filter(|&b| entity_position(ctx.world, b).is_none_or(|p| !p.y.is_finite() || p.y < bottom))
            .collect();
        for bullet in stray_enemy {
            self.invader_bullets.retain(|&b| b != bullet);
            self.physics.destroy_entity(ctx.world, bullet);
        }
    }

    /// Retire a player bullet and account for the shot: kills extend the
    /// sharpshooter streak, anything else resets it.
    pub(super) fn finish_player_shot(&mut self, ctx: &mut GameContext, bullet: EntityId, killed: bool) {
        self.player_bullets.retain(|&b| b != bullet);
        self.physics.destroy_entity(ctx.world, bullet);
        if killed {
            self.shot_streak += 1;
            if self.shot_streak == SHARPSHOOTER_TARGET {
                ctx.achievements.unlock(crate::achievements::SHARPSHOOTER);
            }
        } else {
            self.shot_streak = 0;
        }
    }

    /// The cannon took a hit: explode, clear the incoming volley, and spend
    /// a life. The formation keeps marching — losing the last life ends the
    /// game.
    fn player_hit(&mut self, ctx: &mut GameContext) {
        let pos = self.player.and_then(|p| entity_position(ctx.world, p))
            .unwrap_or(Vec2::new(0.0, PLAYER_Y));
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &crate::effects::player_hit_burst(&theme, self.tex_id));
        self.ripple_grid(pos, GRID_IMPULSE_PLAYER_HIT_STRENGTH, GRID_IMPULSE_PLAYER_HIT_RADIUS);

        self.destroy_all_bullets(ctx.world);
        self.shot_streak = 0;
        self.lives = self.lives.saturating_sub(1);
        if self.lives == 0 {
            self.finish_game(ctx, false);
            return;
        }

        // Respawn at center; the fleet doesn't wait.
        if let Some(player) = self.player {
            self.physics.set_kinematic_target(player, Vec2::new(0.0, PLAYER_Y), 0.0);
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
