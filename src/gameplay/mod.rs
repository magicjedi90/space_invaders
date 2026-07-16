//! Per-frame gameplay orchestration. The pieces live in submodules:
//! formation marching (`formation`), shooting and hit resolution
//! (`combat`), and match lifecycle (`flow`).

mod combat;
mod flow;
mod formation;
mod players;
mod ufo;

#[cfg(test)]
pub(crate) use combat::{invader_fire_rate, pick_shooter_column, player_fire_caps};
#[cfg(test)]
pub(crate) use formation::{fleet_has_landed, march_speed, march_step, MarchOutcome};
#[cfg(test)]
pub(crate) use players::{cannon_spawn_x, coop_defeated, volley_fits};
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

        // Pause gate: while paused the whole match is frozen — no physics
        // step, no march, no fire timers; the overlay is drawn in the UI pass.
        if self.state == GameState::Playing {
            let action = self.pause.update(ctx.players, ctx.input);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx.world); return; }
                PauseAction::ExitGame => { ctx.exit_requested = true; return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                // Keep the frozen scene visible under the pause overlay:
                // re-emit the grid without advancing it (dt 0).
                engine_core::grid::step_and_emit_grid(
                    self.grid.as_mut(), ctx.world, ctx.lines, 0.0, self.debug_colliders,
                );
                return;
            }
        }

        self.update_cannons(ctx);
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

    /// Push a radial shockwave into the deforming grid.
    pub(crate) fn ripple_grid(&mut self, position: Vec2, strength: f32, radius: f32) {
        if let Some(grid) = self.grid.as_mut() {
            grid.apply_impulse(&GridImpulse::Radial { position, strength, radius, attractive: false });
        }
    }

    /// Gameplay sprites only exist on screen outside the menu screens.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = matches!(self.state, GameState::Playing | GameState::GameOver { .. });
        let entities: Vec<EntityId> = self.players.iter().filter_map(|p| p.entity)
            .chain(self.ufo)
            .chain(self.invaders.iter().map(|i| i.entity))
            .chain(self.barrier_blocks.iter().copied())
            .chain(self.players.iter().flat_map(|p| p.bullets.iter().copied()))
            .chain(self.invader_bullets.iter().copied())
            .collect();
        set_sprites_visible(ctx.world, entities, visible);
    }
}
