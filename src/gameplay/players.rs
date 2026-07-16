//! The defending cannons: movement, trigger discipline, and taking a hit.
//! Everything here is parameterized by player index so single player (one
//! merged cannon) and co-op (two independent cannons) share one code path.

use engine_core::prelude::*;

use crate::constants::*;
use crate::types::*;

use super::combat::player_fire_caps;
use super::entity_position;

/// Where cannon `index` spawns for a roster of `player_count`: single player
/// sits at center; co-op splits the two cannons to ∓`CANNON_COOP_OFFSET`.
pub(crate) fn cannon_spawn_x(player_count: usize, index: usize) -> f32 {
    if player_count <= 1 {
        0.0
    } else if index == 0 {
        -CANNON_COOP_OFFSET
    } else {
        CANNON_COOP_OFFSET
    }
}

/// Cannon (and bullet) tint for player `index`: player 1 wears the chaos
/// theme's accent, everyone else the fixed player-2 color.
pub(crate) fn player_color(index: usize, theme: &ChaosTheme) -> Vec4 {
    if index == 0 {
        theme.accent_color
    } else {
        PLAYER2_COLOR
    }
}

/// Whether a fresh volley of `shots` fits under `max_live` given the cannon's
/// own bullets already in flight. Each cannon checks this against its own
/// bullets, so one player firing at cap never blocks the other.
pub(crate) fn volley_fits(live_bullets: usize, shots: usize, max_live: usize) -> bool {
    live_bullets + shots <= max_live
}

/// The co-op match is lost only when every cannon is out of lives. An empty
/// roster (no match in progress) is not a defeat.
pub(crate) fn coop_defeated(lives: &[u32]) -> bool {
    !lives.is_empty() && lives.iter().all(|&l| l == 0)
}

impl SpaceInvadersGame {
    /// Drive every live cannon from its player's input.
    pub(crate) fn update_cannons(&mut self, ctx: &GameContext) {
        for index in 0..self.players.len() {
            self.update_one_cannon(ctx, index);
        }
    }

    /// Move cannon `index`. Keys/stick take over whenever pressed; the mouse
    /// takes over on movement but only drives the lone single-player cannon.
    fn update_one_cannon(&mut self, ctx: &GameContext, index: usize) {
        let Some(entity) = self.players[index].entity else { return };
        let x = entity_position(ctx.world, entity).map(|p| p.x).unwrap_or(0.0);

        let key_dx = self.cannon_axis(ctx, index) * PLAYER_SPEED;
        let mouse_active = index == 0
            && self.mode == GameMode::SinglePlayer
            && ctx.input.mouse_movement_delta().0.abs() > 0.0;

        let new_x = if key_dx != 0.0 {
            x + key_dx * ctx.delta_time
        } else if mouse_active {
            // Window pixels (origin top-left) → world (origin center).
            ctx.input.mouse_position().x - ctx.window_size.x / 2.0
        } else {
            x
        };

        let new_x = new_x.clamp(-PLAYER_MAX_X, PLAYER_MAX_X);
        self.physics.set_kinematic_target(entity, Vec2::new(new_x, PLAYER_Y), 0.0);
    }

    /// Merged horizontal axis for cannon `index`. The single cannon answers
    /// to both players' devices; each co-op cannon answers to its own.
    fn cannon_axis(&self, ctx: &GameContext, index: usize) -> f32 {
        match self.mode {
            GameMode::SinglePlayer => (ctx.players.move_x(PlayerId::P1, ctx.input)
                + ctx.players.move_x(PlayerId::P2, ctx.input))
            .clamp(-1.0, 1.0),
            GameMode::TwoPlayerCoop => ctx.players.move_x(PlayerId(index as u8), ctx.input),
        }
    }

    /// Is the fire button held for cannon `index`? The single cannon fires on
    /// either player's Action1 (Space/mouse or Enter); each co-op cannon on
    /// its own.
    fn fire_held(&self, ctx: &GameContext, index: usize) -> bool {
        match self.mode {
            GameMode::SinglePlayer => {
                ctx.players.is_active(PlayerId::P1, GameAction::Action1, ctx.input)
                    || ctx.players.is_active(PlayerId::P2, GameAction::Action1, ctx.input)
            }
            GameMode::TwoPlayerCoop => {
                ctx.players.is_active(PlayerId(index as u8), GameAction::Action1, ctx.input)
            }
        }
    }

    /// Hold-to-fire for every live cannon; each keeps its own cooldown and
    /// bullet cap. The chaos mode sets the trigger discipline (see
    /// [`player_fire_caps`]).
    pub(crate) fn update_firing(&mut self, ctx: &mut GameContext) {
        for index in 0..self.players.len() {
            self.update_one_cannon_fire(ctx, index);
        }
    }

    fn update_one_cannon_fire(&mut self, ctx: &mut GameContext, index: usize) {
        self.players[index].fire_cooldown =
            (self.players[index].fire_cooldown - ctx.delta_time).max(0.0);

        if !self.fire_held(ctx, index) || self.players[index].fire_cooldown > 0.0 {
            return;
        }

        let (shots, max_live) = player_fire_caps(self.chaos_mode);
        if !volley_fits(self.players[index].bullets.len(), shots, max_live) {
            return;
        }
        let Some(entity) = self.players[index].entity else { return };
        let Some(pos) = entity_position(ctx.world, entity) else { return };

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let color = player_color(index, &theme);
        let muzzle_y = pos.y + PLAYER_H / 2.0 + PLAYER_BULLET_H / 2.0 + 1.0;
        if shots == 2 {
            for dx in [-TWIN_CANNON_OFFSET, TWIN_CANNON_OFFSET] {
                self.spawn_player_bullet(ctx.world, index, Vec2::new(pos.x + dx, muzzle_y), color);
            }
        } else {
            self.spawn_player_bullet(ctx.world, index, Vec2::new(pos.x, muzzle_y), color);
        }
        self.players[index].fire_cooldown = FIRE_COOLDOWN;
    }

    /// Cannon `index` took a hit: explode, clear the incoming volley, and
    /// spend one of that player's lives. At zero lives the cannon is
    /// despawned and the survivor plays on; the match ends only once every
    /// cannon is gone.
    pub(super) fn player_hit(&mut self, ctx: &mut GameContext, index: usize) {
        let spawn_x = cannon_spawn_x(self.players.len(), index);
        let pos = self.players[index]
            .entity
            .and_then(|e| entity_position(ctx.world, e))
            .unwrap_or(Vec2::new(spawn_x, PLAYER_Y));
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles
            .spawn_burst(pos, &crate::effects::player_hit_burst(&theme, self.tex_id));
        self.ripple_grid(pos, GRID_IMPULSE_PLAYER_HIT_STRENGTH, GRID_IMPULSE_PLAYER_HIT_RADIUS);

        self.destroy_all_bullets(ctx.world);
        self.players[index].shot_streak = 0;
        self.players[index].lives = self.players[index].lives.saturating_sub(1);

        if self.players[index].lives == 0 {
            if let Some(entity) = self.players[index].entity.take() {
                self.physics.destroy_entity(ctx.world, entity);
            }
            let lives: Vec<u32> = self.players.iter().map(|p| p.lives).collect();
            if coop_defeated(&lives) {
                self.finish_game(ctx, false);
            }
            return;
        }

        // Survives: respawn at this cannon's home column; the fleet doesn't wait.
        if let Some(entity) = self.players[index].entity {
            self.physics
                .set_kinematic_target(entity, Vec2::new(spawn_x, PLAYER_Y), 0.0);
        }
    }

    /// Retire one of cannon `index`'s bullets and account for the shot: kills
    /// extend that player's sharpshooter streak, anything else resets it.
    pub(super) fn finish_player_shot(
        &mut self,
        ctx: &mut GameContext,
        index: usize,
        bullet: EntityId,
        killed: bool,
    ) {
        self.players[index].bullets.retain(|&b| b != bullet);
        self.physics.destroy_entity(ctx.world, bullet);
        if killed {
            self.players[index].shot_streak += 1;
            if self.players[index].shot_streak == SHARPSHOOTER_TARGET {
                ctx.achievements.unlock(crate::achievements::SHARPSHOOTER);
            }
        } else {
            self.players[index].shot_streak = 0;
        }
    }
}
