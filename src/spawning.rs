//! All entity creation. Every entity gets a `Name` component so the editor
//! hierarchy shows "Invader r0 c3" instead of "Entity 7".

use engine_core::prelude::*;
use crate::constants::*;
use crate::types::{Invader, SpaceInvadersGame};

/// Spawn a player cannon near the bottom of the playfield at horizontal
/// position `x`. Kinematic so gameplay drives it with `set_kinematic_target`
/// (paddle precedent).
pub(crate) fn spawn_player(world: &mut World, tex: u32, color: Vec4, x: f32) -> EntityId {
    world.spawn()
        .with(Name::new("Player"))
        .with(Transform2D::from_parts(Vec2::new(x, PLAYER_Y), 0.0, PLAYER_SCALE))
        // The cannon glows strongly — the signature neon treatment for
        // player-controlled objects.
        .with(Sprite::new(tex).with_color(color).with_emissive(PLAYER_EMISSIVE))
        .with(RigidBody::new_kinematic().with_rotation_locked(true))
        .with(Collider::box_collider(PLAYER_W, PLAYER_H).with_friction(0.0))
        .id()
}

/// Home X of formation column `col` (0-based, left to right) before any
/// march offset. The layout is centered on x = 0.
pub(crate) fn invader_home_x(col: usize) -> f32 {
    let total = INVADER_COLS as f32 * INVADER_W + (INVADER_COLS as f32 - 1.0) * INVADER_GAP_X;
    -(total - INVADER_W) / 2.0 + col as f32 * (INVADER_W + INVADER_GAP_X)
}

/// Home Y of formation row `row` (0-based, top to bottom) before any descent.
pub(crate) fn invader_home_y(row: usize) -> f32 {
    FORMATION_TOP_Y - row as f32 * (INVADER_H + INVADER_GAP_Y)
}

/// Points paid out by an invader in `row`.
pub(crate) fn invader_value(row: usize) -> u32 {
    INVADER_ROW_VALUES[row.min(INVADER_ROWS - 1)]
}

/// Spawn the full formation: `INVADER_ROWS` × `INVADER_COLS` kinematic
/// bodies, tinted per row. The formation marches as one — each frame
/// gameplay retargets every body to its home slot plus the shared offset.
pub(crate) fn spawn_invaders(world: &mut World, tex: u32) -> Vec<Invader> {
    let mut invaders = Vec::with_capacity(INVADER_ROWS * INVADER_COLS);
    for (row, &color) in INVADER_ROW_COLORS.iter().enumerate() {
        for col in 0..INVADER_COLS {
            let pos = Vec2::new(invader_home_x(col), invader_home_y(row));
            let entity = world.spawn()
                .with(Name::new(format!("Invader r{row} c{col}")))
                .with(Transform2D::from_parts(pos, 0.0, INVADER_SCALE))
                .with(Sprite::new(tex).with_color(color).with_emissive(INVADER_EMISSIVE))
                .with(RigidBody::new_kinematic().with_rotation_locked(true))
                .with(Collider::box_collider(INVADER_W, INVADER_H).with_friction(0.0))
                .id();
            invaders.push(Invader { entity, row, col });
        }
    }
    invaders
}

/// Center position of block (`row`, `col`) of the barrier centered at
/// `barrier_x` (row 0 is the top block row).
pub(crate) fn barrier_block_pos(barrier_x: f32, row: usize, col: usize) -> Vec2 {
    let w = BARRIER_BLOCK_COLS as f32 * BARRIER_BLOCK;
    let h = BARRIER_BLOCK_ROWS as f32 * BARRIER_BLOCK;
    Vec2::new(
        barrier_x - (w - BARRIER_BLOCK) / 2.0 + col as f32 * BARRIER_BLOCK,
        BARRIER_Y + (h - BARRIER_BLOCK) / 2.0 - row as f32 * BARRIER_BLOCK,
    )
}

/// Spawn every bunker as a grid of small destructible blocks. Blocks are
/// static sensors: bullets report the contact and game code removes the
/// block — nothing bounces off a bunker.
pub(crate) fn spawn_barriers(world: &mut World, tex: u32, color: Vec4) -> Vec<EntityId> {
    let mut blocks = Vec::with_capacity(BARRIER_COUNT * BARRIER_BLOCK_COLS * BARRIER_BLOCK_ROWS);
    for (i, &x) in BARRIER_XS.iter().enumerate() {
        for row in 0..BARRIER_BLOCK_ROWS {
            for col in 0..BARRIER_BLOCK_COLS {
                let pos = barrier_block_pos(x, row, col);
                let entity = world.spawn()
                    .with(Name::new(format!("Barrier {i} r{row} c{col}")))
                    .with(Transform2D::from_parts(
                        pos, 0.0, Vec2::splat(BARRIER_BLOCK / RENDER_UNIT)))
                    .with(Sprite::new(tex).with_color(color).with_emissive(BARRIER_EMISSIVE))
                    .with(RigidBody::new_static())
                    .with(Collider::box_collider(BARRIER_BLOCK, BARRIER_BLOCK).as_sensor())
                    .id();
                blocks.push(entity);
            }
        }
    }
    blocks
}

impl SpaceInvadersGame {
    /// Spawn one bullet: a dynamic sensor flying straight up or down with a
    /// `Lifetime` safety net (the engine despawns strays automatically).
    fn spawn_bullet(
        &mut self,
        world: &mut World,
        name: &str,
        pos: Vec2,
        size: Vec2,
        velocity: Vec2,
        color: Vec4,
    ) -> EntityId {
        let entity = world.spawn()
            .with(Name::new(name))
            .with(Transform2D::from_parts(pos, 0.0, size / RENDER_UNIT))
            .with(Sprite::new(self.tex_id).with_color(color).with_emissive(BULLET_EMISSIVE))
            .with(RigidBody::new_dynamic()
                .with_gravity_scale(0.0)
                .with_rotation_locked(true)
                .with_linear_damping(0.0)
                .with_angular_damping(0.0))
            .with(Collider::box_collider(size.x, size.y).as_sensor())
            .with(Lifetime::new(BULLET_LIFETIME))
            .id();
        // Buffered-safe on the spawn frame; applied once the body syncs.
        self.physics.set_velocity(entity, velocity, 0.0);
        entity
    }

    pub(crate) fn spawn_player_bullet(
        &mut self,
        world: &mut World,
        player_index: usize,
        pos: Vec2,
        color: Vec4,
    ) {
        let bullet = self.spawn_bullet(
            world,
            "Player Bullet",
            pos,
            Vec2::new(PLAYER_BULLET_W, PLAYER_BULLET_H),
            Vec2::new(0.0, PLAYER_BULLET_SPEED),
            color,
        );
        self.players[player_index].bullets.push(bullet);
    }

    pub(crate) fn spawn_invader_bullet(&mut self, world: &mut World, pos: Vec2) {
        let bullet = self.spawn_bullet(
            world,
            "Invader Bullet",
            pos,
            Vec2::new(INVADER_BULLET_W, INVADER_BULLET_H),
            Vec2::new(0.0, -INVADER_BULLET_SPEED),
            INVADER_BULLET_COLOR,
        );
        self.invader_bullets.push(bullet);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formation_spawns_full_grid() {
        let mut world = World::new();
        let invaders = spawn_invaders(&mut world, 0);
        assert_eq!(invaders.len(), INVADER_ROWS * INVADER_COLS);
    }

    #[test]
    fn formation_is_centered_and_fits_inside_march_bounds() {
        let left_edge = invader_home_x(0) - INVADER_W / 2.0;
        let right_edge = invader_home_x(INVADER_COLS - 1) + INVADER_W / 2.0;
        assert!((left_edge + right_edge).abs() < 0.001, "layout must be symmetric");
        assert!(
            right_edge < MARCH_BOUND_X,
            "formation must have room to march: edge {right_edge} vs bound {MARCH_BOUND_X}"
        );
    }

    #[test]
    fn formation_rows_descend_and_start_above_barriers() {
        assert!(invader_home_y(0) > invader_home_y(INVADER_ROWS - 1));
        let lowest = invader_home_y(INVADER_ROWS - 1) - INVADER_H / 2.0;
        assert!(
            lowest > BARRIER_Y + BARRIER_BLOCK_ROWS as f32 * BARRIER_BLOCK,
            "fresh formation must not start inside the barriers: {lowest}"
        );
    }

    #[test]
    fn top_row_invaders_pay_the_most() {
        assert_eq!(invader_value(0), 30);
        assert_eq!(invader_value(INVADER_ROWS - 1), 10);
        for row in 1..INVADER_ROWS {
            assert!(invader_value(row - 1) >= invader_value(row));
        }
        // Out-of-range rows degrade to the minimum payout, never panic.
        assert_eq!(invader_value(99), invader_value(INVADER_ROWS - 1));
    }

    #[test]
    fn invaders_chomp_barriers_before_they_can_land() {
        let bottom = barrier_block_pos(0.0, BARRIER_BLOCK_ROWS - 1, 0).y - BARRIER_BLOCK / 2.0;
        // The invasion line sits below the barriers: a descending fleet
        // chews through the bunkers before the game can end.
        assert!(INVASION_Y < bottom, "invasion line must be below the barriers");
        assert!(bottom > PLAYER_Y + PLAYER_H / 2.0, "barriers must sit above the player");
    }

    #[test]
    fn barrier_blocks_span_each_bunker_symmetrically() {
        let mut world = World::new();
        let blocks = spawn_barriers(&mut world, 0, Vec4::ONE);
        assert_eq!(blocks.len(), BARRIER_COUNT * BARRIER_BLOCK_COLS * BARRIER_BLOCK_ROWS);

        let left = barrier_block_pos(100.0, 0, 0).x;
        let right = barrier_block_pos(100.0, 0, BARRIER_BLOCK_COLS - 1).x;
        assert!(((left - 100.0) + (right - 100.0)).abs() < 0.001);
    }
}
