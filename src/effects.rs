//! Visual effect presets — particle configs.
//!
//! Centralizes the look of each event (invader kill, player hit, barrier
//! chip) so tuning happens in one place. The deforming grid uses the
//! engine's `default_playfield_grid` preset directly.

use engine_core::prelude::*;

/// Omnidirectional shatter when an invader dies, tinted to its row color.
pub(crate) fn invader_death_burst(color: Vec4, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (26.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.25, 0.6)
        .with_speed(100.0, 320.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(6.0, 0.5)
        .with_drag(2.2)
        .with_emissive(2.2)
        .with_texture(tex)
}

/// Large explosion when the player cannon is hit.
pub(crate) fn player_hit_burst(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let color = Vec4::new(1.0, 0.35, 0.25, 1.0);
    let count = (70.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.4, 0.9)
        .with_speed(120.0, 480.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(9.0, 0.5)
        .with_drag(1.7)
        .with_emissive(2.8)
        .with_texture(tex)
}

/// Small chip of debris when a bullet (or a marching invader) eats a
/// barrier block.
pub(crate) fn barrier_chip_burst(color: Vec4, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (10.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.15, 0.35)
        .with_speed(80.0, 200.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(4.0, 0.5)
        .with_drag(2.8)
        .with_emissive(1.6)
        .with_texture(tex)
}

/// Tiny flash where two bullets cancel each other mid-air.
pub(crate) fn bullet_cancel_burst(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let color = Vec4::new(1.0, 0.95, 0.6, 1.0);
    let count = (14.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.12, 0.3)
        .with_speed(90.0, 240.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(4.0, 0.5)
        .with_drag(2.6)
        .with_emissive(2.4)
        .with_texture(tex)
}
