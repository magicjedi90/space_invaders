//! The mystery ship: every now and then a UFO crosses the top lane and
//! pays a hash-drawn bonus when shot. Purely optional target — it never
//! fires and never lands.

use engine_core::prelude::*;

use crate::constants::*;
use crate::types::*;

use super::entity_position;

// Distinct hash streams so UFO draws don't correlate with invader fire
// (which draws on the bare frame count).
const SPAWN_SALT: u32 = 4093;
const DIR_SALT: u32 = 8191;
const BONUS_SALT: u32 = 3301;

/// Where a UFO enters and which way it flies, from a pseudo-random draw:
/// even → left edge flying right, odd → right edge flying left. It spawns
/// one full ship-width off screen so it slides in instead of popping.
pub(crate) fn ufo_entry(rand: u32) -> (f32, f32) {
    let dir = if rand.is_multiple_of(2) { 1.0 } else { -1.0 };
    (-dir * (WIN_W / 2.0 + UFO_W), dir)
}

/// A UFO is done once it has fully crossed the far edge.
pub(crate) fn ufo_offscreen(x: f32, dir: f32) -> bool {
    x * dir > WIN_W / 2.0 + UFO_W
}

/// The mystery bonus for a kill, drawn from the classic table.
pub(crate) fn ufo_bonus(rand: u32) -> u32 {
    UFO_BONUS_VALUES[rand as usize % UFO_BONUS_VALUES.len()]
}

impl SpaceInvadersGame {
    /// Spawn draws, flight, and despawn for the mystery ship; also ticks
    /// the bonus HUD flash.
    pub(crate) fn update_ufo(&mut self, ctx: &mut GameContext) {
        self.ufo_flash = (self.ufo_flash - ctx.delta_time).max(0.0);

        let Some(ufo) = self.ufo else {
            // No ship in flight: roll this frame's spawn draw.
            if hash_f32(self.frame_count.wrapping_add(SPAWN_SALT)) < UFO_SPAWN_RATE * ctx.delta_time {
                let (start_x, dir) = ufo_entry(hash_u32(self.frame_count.wrapping_add(DIR_SALT)));
                self.ufo_dir = dir;
                self.ufo = Some(self.spawn_ufo(ctx.world, Vec2::new(start_x, UFO_Y)));
            }
            return;
        };

        let Some(pos) = entity_position(ctx.world, ufo) else {
            self.ufo = None;
            return;
        };
        let x = pos.x + self.ufo_dir * UFO_SPEED * ctx.delta_time;
        if ufo_offscreen(x, self.ufo_dir) {
            self.destroy_ufo(ctx.world);
        } else {
            self.physics.set_kinematic_target(ufo, Vec2::new(x, UFO_Y), 0.0);
        }
    }

    pub(crate) fn spawn_ufo(&mut self, world: &mut World, pos: Vec2) -> EntityId {
        world.spawn()
            .with(Name::new("UFO"))
            .with(Transform2D::from_parts(pos, 0.0, UFO_SCALE))
            .with(Sprite::new(self.tex_id).with_color(UFO_COLOR).with_emissive(UFO_EMISSIVE))
            .with(RigidBody::new_kinematic().with_rotation_locked(true))
            .with(Collider::box_collider(UFO_W, UFO_H).with_friction(0.0))
            .id()
    }

    /// Pay out a player bullet that hit the mystery ship: hash-drawn bonus,
    /// HUD flash, big burst — and it counts as a kill shot for the streak.
    pub(crate) fn check_ufo_hit(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let Some(ufo) = self.ufo else { return };
        let hit = self.player_bullets.iter().copied().find(|&bullet| {
            collisions.iter().any(|c| c.event.started && c.event.involves(bullet, ufo))
        });
        let Some(bullet) = hit else { return };

        let bonus = ufo_bonus(hash_u32(self.frame_count.wrapping_add(BONUS_SALT)));
        self.score += bonus;
        self.ufo_flash_bonus = bonus;
        self.ufo_flash = UFO_FLASH_SECS;

        if let Some(pos) = entity_position(ctx.world, ufo) {
            let theme = ChaosTheme::for_mode(self.chaos_mode);
            ctx.particles.spawn_burst(
                pos, &crate::effects::ufo_death_burst(&theme, self.tex_id));
            self.ripple_grid(pos, GRID_IMPULSE_KILL_STRENGTH, GRID_IMPULSE_KILL_RADIUS);
        }
        self.destroy_ufo(ctx.world);
        self.finish_player_shot(ctx, bullet, true);
    }

    pub(crate) fn destroy_ufo(&mut self, world: &mut World) {
        if let Some(ufo) = self.ufo.take() {
            self.physics.destroy_entity(world, ufo);
        }
    }
}
