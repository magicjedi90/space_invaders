//! Per-frame gameplay orchestration. The pieces live in submodules:
//! formation marching (`formation`), shooting and hit resolution
//! (`combat`), and match lifecycle (`flow`).

mod combat;
mod flow;
mod formation;
mod ufo;

#[cfg(test)]
pub(crate) use combat::{invader_fire_rate, pick_shooter_column, player_fire_caps};
#[cfg(test)]
pub(crate) use formation::{march_speed, march_step, MarchOutcome};
#[cfg(test)]
pub(crate) use ufo::{ufo_bonus, ufo_entry, ufo_offscreen};

use engine_core::prelude::*;
use crate::types::*;

/// Axis-aligned overlap test between two rectangles given centers and half
/// extents. All game-side collision fallbacks (barrier chomping, bullet
/// cancels, invader-vs-player) use this — rapier does not report
/// kinematic-vs-static or kinematic-vs-kinematic pairs.
pub(crate) fn rects_overlap(center_a: Vec2, half_a: Vec2, center_b: Vec2, half_b: Vec2) -> bool {
    (center_a.x - center_b.x).abs() <= half_a.x + half_b.x
        && (center_a.y - center_b.y).abs() <= half_a.y + half_b.y
}

pub(crate) fn entity_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world.get::<Transform2D>(entity).map(|t| t.position)
}

impl SpaceInvadersGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        // F1 toggles the collider debug overlay. Magenta outlines render on
        // top of sprites so any sprite-vs-collider mismatch is obvious.
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        self.update_player_movement(ctx);
        self.physics.update(ctx.world, ctx.delta_time);
        // Expired bullets despawn here; the physics system garbage-collects
        // their rapier bodies on its next update.
        self.lifetimes.update(ctx.world, ctx.delta_time);

        // Drain this frame's collision events once (take = the buffer is
        // consumed, not borrowed). Every consumer below shares this Vec.
        let collisions: Vec<CollisionData> = self.physics.take_collision_events();

        self.handle_state_input(ctx);
        if self.state == GameState::Playing {
            self.update_firing(ctx);
            self.update_invader_fire(ctx);
            self.update_ufo(ctx);
            self.march_formation(ctx);
            self.resolve_bullet_hits(ctx, &collisions);
            self.check_ufo_hit(ctx, &collisions);
            self.cancel_crossing_bullets(ctx);
            self.cull_stray_bullets(ctx);
            self.check_invasion(ctx);
            self.check_win_condition(ctx);
        }

        // Step + render the deforming grid after gameplay so it reacts to
        // this frame's events; collider outlines overlay when toggled.
        step_and_emit_grid(
            self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders,
        );
    }

    /// Move the cannon from keyboard or mouse. Mouse takes over whenever it
    /// moves; keys take over whenever they're pressed.
    fn update_player_movement(&mut self, ctx: &GameContext) {
        let Some(player) = self.player else { return };
        let x = entity_position(ctx.world, player).map(|p| p.x).unwrap_or(0.0);

        let left = ctx.input.is_key_pressed(KeyCode::ArrowLeft)
            || ctx.input.is_key_pressed(KeyCode::KeyA);
        let right = ctx.input.is_key_pressed(KeyCode::ArrowRight)
            || ctx.input.is_key_pressed(KeyCode::KeyD);
        let key_dx = match (left, right) {
            (true, false) => -crate::constants::PLAYER_SPEED,
            (false, true) => crate::constants::PLAYER_SPEED,
            _ => 0.0,
        };

        let mouse_moved = ctx.input.mouse_movement_delta().0.abs() > 0.0;
        let new_x = if key_dx != 0.0 {
            x + key_dx * ctx.delta_time
        } else if mouse_moved {
            // Window pixels (origin top-left) → world (origin center).
            ctx.input.mouse_position().x - ctx.window_size.x / 2.0
        } else {
            x
        };

        let new_x = new_x.clamp(-crate::constants::PLAYER_MAX_X, crate::constants::PLAYER_MAX_X);
        self.physics.set_kinematic_target(
            player, Vec2::new(new_x, crate::constants::PLAYER_Y), 0.0);
    }

    /// Push a radial shockwave into the deforming grid.
    pub(crate) fn ripple_grid(&mut self, position: Vec2, strength: f32, radius: f32) {
        if let Some(grid) = self.grid.as_mut() {
            grid.apply_impulse(&GridImpulse::Radial { position, strength, radius, attractive: false });
        }
    }

    /// Gameplay sprites only exist on screen outside the menu screens.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = matches!(self.state, GameState::Playing | GameState::GameOver { .. });
        let entities: Vec<EntityId> = self.player.into_iter()
            .chain(self.ufo)
            .chain(self.invaders.iter().map(|i| i.entity))
            .chain(self.barrier_blocks.iter().copied())
            .chain(self.player_bullets.iter().copied())
            .chain(self.invader_bullets.iter().copied())
            .collect();
        set_sprites_visible(ctx.world, entities, visible);
    }
}
