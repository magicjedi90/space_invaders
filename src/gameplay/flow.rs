//! Match lifecycle: state-machine input during play, starting/ending a
//! match, and pushing the chaos theme onto live entities.

use engine_core::prelude::*;

use crate::constants::*;
use crate::spawning;
use crate::types::*;

impl SpaceInvadersGame {
    /// Keys that change the game state while the simulation screens are up.
    pub(crate) fn handle_state_input(&mut self, ctx: &mut GameContext) {
        match &self.state {
            GameState::Playing => {
                if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                }
            }
            GameState::GameOver { .. } => {
                if ctx.input.is_key_just_pressed(KeyCode::Space)
                    || ctx.input.is_key_just_pressed(KeyCode::Enter)
                {
                    self.start_game(ctx);
                } else if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                }
            }
            _ => {}
        }
    }

    /// Reset score/lives, rebuild the fleet and barriers, and start the
    /// march. Called from mode select and from game-over restart.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.score = 0;
        self.lives = STARTING_LIVES;
        self.shot_streak = 0;
        self.fire_cooldown = 0.0;
        self.march_dir = 1.0;
        self.formation_offset = Vec2::ZERO;

        self.destroy_all_bullets(ctx.world);
        self.destroy_ufo(ctx.world);
        self.ufo_flash = 0.0;
        self.destroy_fleet_and_barriers(ctx.world);
        self.invaders = spawning::spawn_invaders(ctx.world, self.tex_id);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        self.barrier_blocks = spawning::spawn_barriers(ctx.world, self.tex_id, theme.structure_color);

        self.apply_theme(ctx.world);
        if let Some(player) = self.player {
            self.physics.set_kinematic_target(player, Vec2::new(0.0, PLAYER_Y), 0.0);
        }
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
        for bullet in self.player_bullets.drain(..).chain(self.invader_bullets.drain(..)) {
            self.physics.destroy_entity(world, bullet);
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
        if let Some(player) = self.player {
            if let Some(s) = world.get_mut::<Sprite>(player) { s.color = theme.accent_color; }
        }
        self.grid = Some(default_playfield_grid(&theme));
    }
}
