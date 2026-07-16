//! Match lifecycle: state-machine input during play, starting/ending a
//! match, and pushing the chaos theme onto live entities.

use engine_core::prelude::*;

use crate::spawning;
use crate::types::*;

use super::players::{cannon_spawn_x, player_color};

impl SpaceInvadersGame {
    /// Actions that change the game state while the simulation screens are up.
    /// Either player's Menu (Escape/pad Start) or Action1 (Space/Enter/pad A)
    /// counts, so any controller can pause or restart.
    pub(crate) fn handle_state_input(&mut self, ctx: &mut GameContext) {
        match &self.state {
            GameState::Playing => {
                if ctx.players.just_activated_any(GameAction::Menu, ctx.input) {
                    self.reset_to_title(ctx.world);
                }
            }
            GameState::GameOver { .. } => {
                if ctx.players.just_activated_any(GameAction::Action1, ctx.input) {
                    self.start_game(ctx);
                } else if ctx.players.just_activated_any(GameAction::Menu, ctx.input) {
                    self.reset_to_title(ctx.world);
                }
            }
            _ => {}
        }
    }

    /// Build a fresh roster for the chosen mode, rebuild the fleet and
    /// barriers, spawn the cannons, and start the march. Called from mode
    /// select and from game-over restart.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.march_dir = 1.0;
        self.formation_offset = Vec2::ZERO;

        // Rebuild the cannons for this mode: fresh scores, lives, and bodies.
        let count = self.mode.player_count();
        self.despawn_cannons(ctx.world);
        self.players = (0..count).map(|_| PlayerState::fresh()).collect();

        self.destroy_all_bullets(ctx.world);
        self.destroy_ufo(ctx.world);
        self.ufo_flash = 0.0;
        self.destroy_fleet_and_barriers(ctx.world);
        self.invaders = spawning::spawn_invaders(ctx.world, self.tex_id);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        self.barrier_blocks = spawning::spawn_barriers(ctx.world, self.tex_id, theme.structure_color);

        for index in 0..count {
            let x = cannon_spawn_x(count, index);
            let color = player_color(index, &theme);
            let entity = spawning::spawn_player(ctx.world, self.tex_id, color, x);
            self.players[index].entity = Some(entity);
        }

        self.apply_theme(ctx.world);
        self.state = GameState::Playing;
    }

    /// End the match, win or lose. Entities stay on screen behind the
    /// game-over overlay; the next start rebuilds them.
    pub(crate) fn finish_game(&mut self, ctx: &mut GameContext, won: bool) {
        self.destroy_all_bullets(ctx.world);
        self.destroy_ufo(ctx.world);
        self.state = GameState::GameOver { won };
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_all_bullets(world);
        self.destroy_ufo(world);
        self.state = GameState::TitleScreen { selection: 0 };
    }

    pub(crate) fn destroy_all_bullets(&mut self, world: &mut World) {
        let mut bullets: Vec<EntityId> = self.invader_bullets.drain(..).collect();
        for player in &mut self.players {
            bullets.append(&mut player.bullets);
        }
        for bullet in bullets {
            self.physics.destroy_entity(world, bullet);
        }
    }

    /// Destroy every cannon body and forget it. Called when rebuilding the
    /// roster for a new match.
    fn despawn_cannons(&mut self, world: &mut World) {
        let entities: Vec<EntityId> = self.players.iter_mut()
            .filter_map(|p| p.entity.take())
            .collect();
        for entity in entities {
            self.physics.destroy_entity(world, entity);
        }
    }

    fn destroy_fleet_and_barriers(&mut self, world: &mut World) {
        for invader in self.invaders.drain(..) {
            self.physics.destroy_entity(world, invader.entity);
        }
        for block in self.barrier_blocks.drain(..) {
            self.physics.destroy_entity(world, block);
        }
    }

    /// Push the current `chaos_mode`'s look onto the live entities:
    /// background tint, player color, and a fresh grid.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(bg) = self.background {
            if let Some(s) = world.get_mut::<Sprite>(bg) { s.color = theme.bg_color; }
        }
        for index in 0..self.players.len() {
            let color = player_color(index, &theme);
            if let Some(entity) = self.players[index].entity {
                if let Some(s) = world.get_mut::<Sprite>(entity) { s.color = color; }
            }
        }
        self.grid = Some(default_playfield_grid(&theme));
    }
}
