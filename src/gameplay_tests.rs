//! Headless tests for gameplay logic: pure formation/combat math plus
//! physics simulations using the exact entity recipes the game spawns.

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::{
    invader_fire_rate, march_speed, march_step, pick_shooter_column, player_fire_caps,
    rects_overlap, MarchOutcome,
};
use crate::menu::mode_hint;
use crate::spawning::{invader_home_x, spawn_barriers, spawn_invaders, spawn_player, barrier_block_pos};
use crate::types::SpaceInvadersGame;

// --- Formation math ---

#[test]
fn march_step_advances_without_descending_mid_field() {
    let out = march_step(0.0, 1.0, 5.0, -100.0, 100.0);
    assert_eq!(out, MarchOutcome { offset_x: 5.0, dir: 1.0, descended: false });
}

#[test]
fn march_step_bounces_off_right_bound_and_descends() {
    let live_max = 100.0;
    let near = MARCH_BOUND_X - live_max - 1.0; // 1px short of the bound
    let out = march_step(near, 1.0, 5.0, -100.0, live_max);
    assert!(out.descended, "crossing the bound must trigger a descent");
    assert_eq!(out.dir, -1.0, "direction must reverse");
    assert!((out.offset_x - (MARCH_BOUND_X - live_max)).abs() < 0.001, "offset clamps to the bound");
}

#[test]
fn march_step_bounces_off_left_bound_and_descends() {
    let live_min = -100.0;
    let near = -MARCH_BOUND_X - live_min + 1.0;
    let out = march_step(near, -1.0, 5.0, live_min, 100.0);
    assert!(out.descended);
    assert_eq!(out.dir, 1.0);
    assert!((out.offset_x - (-MARCH_BOUND_X - live_min)).abs() < 0.001);
}

#[test]
fn march_step_uses_live_extremes_so_thinned_fleet_marches_farther() {
    // Only the center column survives: its home x is ~0, so the offset can
    // grow all the way to the bound before bouncing.
    let out = march_step(MARCH_BOUND_X - 5.0, 1.0, 1.0, 0.0, 0.0);
    assert!(!out.descended, "a thin fleet keeps marching where a full one would bounce");
}

#[test]
fn march_speed_ramps_from_base_to_max_as_fleet_thins() {
    let total = INVADER_ROWS * INVADER_COLS;
    assert_eq!(march_speed(total, total, false), MARCH_SPEED_BASE);
    assert_eq!(march_speed(1, total, false), MARCH_SPEED_MAX);
    let mid = march_speed(total / 2, total, false);
    assert!(mid > MARCH_SPEED_BASE && mid < MARCH_SPEED_MAX);
}

#[test]
fn march_speed_insane_mode_multiplies() {
    let total = INVADER_ROWS * INVADER_COLS;
    assert_eq!(
        march_speed(total, total, true),
        MARCH_SPEED_BASE * INSANE_MARCH_MULT
    );
}

#[test]
fn march_speed_degenerate_counts_never_panic() {
    assert!(march_speed(0, 0, false) > 0.0);
    assert!(march_speed(5, 3, false) > 0.0); // alive > total clamps
}

// --- Combat math ---

#[test]
fn pick_shooter_column_wraps_over_live_columns() {
    let live = [2usize, 5, 7];
    assert_eq!(pick_shooter_column(&live, 0), 2);
    assert_eq!(pick_shooter_column(&live, 1), 5);
    assert_eq!(pick_shooter_column(&live, 2), 7);
    assert_eq!(pick_shooter_column(&live, 3), 2);
}

#[test]
fn rects_overlap_detects_touch_and_separation() {
    let half = Vec2::new(10.0, 10.0);
    assert!(rects_overlap(Vec2::ZERO, half, Vec2::new(19.0, 0.0), half));
    assert!(rects_overlap(Vec2::ZERO, half, Vec2::new(20.0, 0.0), half), "edge touch counts");
    assert!(!rects_overlap(Vec2::ZERO, half, Vec2::new(21.0, 0.0), half));
    assert!(!rects_overlap(Vec2::ZERO, half, Vec2::new(0.0, 25.0), half));
}

#[test]
fn player_fire_caps_per_chaos_mode() {
    // Normal: the classic single bullet in flight.
    assert_eq!(player_fire_caps(ChaosMode::Normal), (1, MAX_PLAYER_BULLETS));
    // Insane: one barrel, 4 shots on screen against the faster march.
    assert_eq!(player_fire_caps(ChaosMode::Insane), (1, INSANE_MAX_PLAYER_BULLETS));
    // Ridiculous: twin cannons, one volley in flight.
    assert_eq!(player_fire_caps(ChaosMode::Ridiculous), (2, RIDICULOUS_MAX_PLAYER_BULLETS));
    // Insiculous faces both fleet buffs: twin cannons AND stacked volleys.
    assert_eq!(player_fire_caps(ChaosMode::Insiculous), (2, INSICULOUS_MAX_PLAYER_BULLETS));
    // A volley must always fit under its own cap.
    for &mode in &ChaosMode::ALL {
        let (shots, max_live) = player_fire_caps(mode);
        assert!(shots <= max_live, "{mode:?} volley larger than its cap");
    }
}

/// The split of fleet buffs: Insane owns march speed, Ridiculous owns fire
/// rate, Insiculous gets both, Normal gets neither.
#[test]
fn fleet_buffs_split_across_chaos_modes() {
    let total = INVADER_ROWS * INVADER_COLS;
    let base_speed = march_speed(total, total, false);
    let speed = |m: ChaosMode| march_speed(total, total, m.is_insane());

    assert_eq!(speed(ChaosMode::Normal), base_speed);
    assert_eq!(invader_fire_rate(ChaosMode::Normal), INVADER_FIRE_RATE);

    assert!(speed(ChaosMode::Insane) > base_speed, "Insane must march faster");
    assert_eq!(invader_fire_rate(ChaosMode::Insane), INVADER_FIRE_RATE,
        "Insane must NOT fire faster — that's Ridiculous's buff");

    assert_eq!(speed(ChaosMode::Ridiculous), base_speed,
        "Ridiculous must NOT march faster — that's Insane's buff");
    assert!(invader_fire_rate(ChaosMode::Ridiculous) > INVADER_FIRE_RATE);

    assert!(speed(ChaosMode::Insiculous) > base_speed);
    assert!(invader_fire_rate(ChaosMode::Insiculous) > INVADER_FIRE_RATE);
}

#[test]
fn every_mode_has_a_hint() {
    for mode in ChaosMode::ALL {
        assert!(!mode_hint(mode).is_empty(), "{mode:?} needs a mode-select hint");
    }
}

// --- Physics simulations (exact game entity recipes) ---

/// A player bullet must report a started contact against a kinematic
/// invader body — the dynamic-sensor-vs-kinematic pair the whole game
/// hangs off.
#[test]
fn player_bullet_registers_hit_on_kinematic_invader() {
    let mut game = SpaceInvadersGame::default();
    let mut world = World::new();

    let invaders = spawn_invaders(&mut world, 0);
    let target = &invaders[0];
    let x = invader_home_x(target.col);

    game.spawn_player_bullet(&mut world, Vec2::new(x, FORMATION_TOP_Y - 80.0), Vec4::ONE);
    let bullet = game.player_bullets[0];

    let mut hit = false;
    for _ in 0..120 {
        game.physics.update(&mut world, 1.0 / 60.0);
        let events = game.physics.take_collision_events();
        if events.iter().any(|c| c.event.started && c.event.involves(bullet, target.entity)) {
            hit = true;
            break;
        }
    }
    assert!(hit, "player bullet never registered a contact with the invader");
}

/// An invader bullet must report a started contact against the kinematic
/// player cannon.
#[test]
fn invader_bullet_registers_hit_on_player() {
    let mut game = SpaceInvadersGame::default();
    let mut world = World::new();

    let player = spawn_player(&mut world, 0, Vec4::ONE);
    game.spawn_invader_bullet(&mut world, Vec2::new(0.0, PLAYER_Y + 100.0));
    let bullet = game.invader_bullets[0];

    let mut hit = false;
    for _ in 0..120 {
        game.physics.update(&mut world, 1.0 / 60.0);
        let events = game.physics.take_collision_events();
        if events.iter().any(|c| c.event.started && c.event.involves(bullet, player)) {
            hit = true;
            break;
        }
    }
    assert!(hit, "invader bullet never registered a contact with the player");
}

/// Bullets must report contacts against the static sensor barrier blocks
/// (sensor-vs-sensor intersection events).
#[test]
fn player_bullet_registers_hit_on_barrier_block() {
    let mut game = SpaceInvadersGame::default();
    let mut world = World::new();

    let blocks = spawn_barriers(&mut world, 0, Vec4::ONE);
    // Aim straight up under the first block of the first bunker.
    let block_pos = barrier_block_pos(BARRIER_XS[0], BARRIER_BLOCK_ROWS - 1, 0);
    game.spawn_player_bullet(
        &mut world, Vec2::new(block_pos.x, block_pos.y - 60.0), Vec4::ONE);
    let bullet = game.player_bullets[0];

    let mut hit_block = None;
    for _ in 0..120 {
        game.physics.update(&mut world, 1.0 / 60.0);
        let events = game.physics.take_collision_events();
        if let Some(c) = events.iter().find(|c| {
            c.event.started && blocks.iter().any(|&b| c.event.involves(bullet, b))
        }) {
            hit_block = c.event.other(bullet);
            break;
        }
    }
    assert!(hit_block.is_some(), "player bullet never registered a contact with a barrier block");
}

/// The `Lifetime` safety net must despawn a stray bullet entirely.
#[test]
fn stray_bullet_expires_via_lifetime_system() {
    let mut game = SpaceInvadersGame::default();
    let mut world = World::new();

    game.spawn_player_bullet(&mut world, Vec2::ZERO, Vec4::ONE);
    let bullet = game.player_bullets[0];
    assert!(world.get::<Lifetime>(bullet).is_some(), "bullets must carry a Lifetime");

    let frames = (BULLET_LIFETIME * 60.0) as usize + 10;
    for _ in 0..frames {
        game.physics.update(&mut world, 1.0 / 60.0);
        game.lifetimes.update(&mut world, 1.0 / 60.0);
    }
    assert!(
        !world.entities().contains(&bullet),
        "expired bullet must be removed from the world"
    );
}
