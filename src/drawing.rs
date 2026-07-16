use engine_core::prelude::*;
use crate::achievements::DISPLAY_SECTIONS;
use crate::menu::mode_hint;
use crate::types::*;

impl SpaceInvadersGame {
    fn menu_style(&self) -> MenuStyle {
        MenuStyle::from_theme(&ChaosTheme::for_mode(self.chaos_mode))
    }

    pub(crate) fn draw_ui(&self, ctx: &mut GameContext) {
        match &self.state {
            GameState::TitleScreen { selection } => self.draw_title(ctx, *selection),
            GameState::ModeSelect { selection } => self.draw_mode_select(ctx, *selection),
            GameState::Achievements => self.draw_achievements(ctx),
            _ => self.draw_gameplay(ctx),
        }
    }

    fn draw_title(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = MenuPanel::new("INSICULOUS INVADERS", ctx.window_size / 2.0, 380.0, 4);
        let mut y = panel.begin(ctx.ui, &style);
        let items = ["1 Player", "2 Player Co-op", "Achievements", "Exit"];
        for (i, item) in items.iter().enumerate() {
            y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
        }
        panel.hint(ctx.ui, "W/S or D-Pad navigate - SPACE or (A) confirm", &style);
    }

    fn draw_mode_select(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = MenuPanel::new("SELECT CHAOS MODE", ctx.window_size / 2.0, 400.0, ChaosMode::ALL.len());
        let mut y = panel.begin(ctx.ui, &style);
        for (i, &mode) in ChaosMode::ALL.iter().enumerate() {
            // Each entry glows in its chaos mode's banner color.
            let c = ChaosTheme::for_mode(mode).banner_color;
            y = panel.item_colored(ctx.ui, y, mode.label(), c, i as u8 == selection, &style);
        }
        panel.hint(
            ctx.ui,
            mode_hint(ChaosMode::ALL[selection as usize % ChaosMode::ALL.len()]),
            &style,
        );
    }

    fn draw_achievements(&self, ctx: &mut GameContext) {
        let style = self.menu_style();
        let cx = ctx.window_size.x / 2.0;
        let total = ctx.achievements.total();
        let unlocked = ctx.achievements.unlocked_count();

        // Tall window; the section list draws left-aligned inside it.
        let panel = MenuPanel::new("ACHIEVEMENTS", ctx.window_size / 2.0, ctx.window_size.x - 120.0, 15);
        let first_y = panel.begin(ctx.ui, &style);
        let rect = panel.panel_rect();
        ctx.ui.label_centered(
            &format!("{unlocked} / {total} unlocked"),
            Vec2::new(cx, first_y - 8.0),
        );

        let left = rect.x + 28.0;
        let mut y = first_y + 18.0;

        let locked_color = Color::new(0.45, 0.45, 0.5, 1.0);
        let unlocked_color = Color::new(1.0, 0.85, 0.25, 1.0);
        let desc_color = Color::new(0.75, 0.75, 0.8, 1.0);
        let header_color = Color::new(0.6, 0.75, 1.0, 1.0);

        for (section, ids) in DISPLAY_SECTIONS {
            ctx.ui.label_styled(section, Vec2::new(left, y), header_color, 16.0);
            y += 22.0;
            for id in *ids {
                let is_unlocked = ctx.achievements.is_unlocked(id);
                // Registry always has entries for these ids (registered in init).
                let Some(ach) = ctx.achievements.get(id) else { continue };

                let (marker, name_color) = if is_unlocked {
                    ("[X]", unlocked_color)
                } else {
                    ("[ ]", locked_color)
                };

                ctx.ui.label_styled(
                    &format!("{marker} {}", ach.name),
                    Vec2::new(left + 8.0, y),
                    name_color,
                    14.0,
                );
                ctx.ui.label_styled(&ach.description, Vec2::new(left + 52.0, y + 16.0), desc_color, 12.0);
                y += 36.0;
            }
            y += 6.0;
        }

        panel.hint(ctx.ui, "ESC or SPACE to go back", &style);
    }

    fn draw_gameplay(&self, ctx: &mut GameContext) {
        let cx = ctx.window_size.x / 2.0;
        let cy = ctx.window_size.y / 2.0;

        self.draw_scoreboard(ctx);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(banner) = theme.banner_text {
            let color = Color::new(theme.banner_color.x, theme.banner_color.y, theme.banner_color.z, theme.banner_color.w);
            ctx.ui.label_centered_styled(banner, Vec2::new(cx, ctx.window_size.y - 24.0), color, 16.0);
        }

        // Streak HUD shows the best streak going across the live cannons.
        let streak = self.players.iter().map(|p| p.shot_streak).max().unwrap_or(0);
        if streak >= 3 {
            ctx.ui.label_centered(&format!("STREAK x{streak}"), Vec2::new(cx, 48.0));
        }

        if self.ufo_flash > 0.0 {
            let c = crate::constants::UFO_COLOR;
            ctx.ui.label_centered_styled(
                &format!("UFO +{}", self.ufo_flash_bonus),
                Vec2::new(cx, 72.0),
                Color::new(c.x, c.y, c.z, c.w),
                16.0,
            );
        }

        if let GameState::GameOver { won } = &self.state {
            let msg = if *won { "INVASION REPELLED!" } else { "EARTH HAS FALLEN" };
            let style = self.menu_style();
            let panel = MenuPanel::new(msg, Vec2::new(cx, cy), 340.0, 2);
            let mut y = panel.begin(ctx.ui, &style);
            y = panel.line(ctx.ui, y, &self.final_score_line(), &style);
            panel.line(ctx.ui, y, "SPACE to play again", &style);
            panel.hint(ctx.ui, "ESC for title screen", &style);
        }

        if self.pause.is_active() {
            let style = self.menu_style();
            self.pause.draw(ctx.ui, ctx.window_size, &style);
        }
    }

    /// Score/lives HUD. Single player keeps the classic centered top row;
    /// co-op splits P1 to the left and P2 to the right.
    fn draw_scoreboard(&self, ctx: &mut GameContext) {
        let right_x = ctx.window_size.x - 140.0;
        match self.mode {
            GameMode::SinglePlayer => {
                let p = &self.players[0];
                ctx.ui.label(&format!("SCORE {}", p.score), Vec2::new(40.0, 16.0));
                ctx.ui.label(&lives_text(p.lives), Vec2::new(right_x, 16.0));
            }
            GameMode::TwoPlayerCoop => {
                let p1 = &self.players[0];
                ctx.ui.label(&format!("P1 {}", p1.score), Vec2::new(40.0, 16.0));
                ctx.ui.label(&lives_text(p1.lives), Vec2::new(40.0, 36.0));
                if let Some(p2) = self.players.get(1) {
                    ctx.ui.label(&format!("P2 {}", p2.score), Vec2::new(right_x, 16.0));
                    ctx.ui.label(&lives_text(p2.lives), Vec2::new(right_x, 36.0));
                }
            }
        }
    }

    /// Game-over final-score line: one score in single player, both in co-op.
    fn final_score_line(&self) -> String {
        match self.mode {
            GameMode::SinglePlayer => {
                format!("Final score: {}", self.players[0].score)
            }
            GameMode::TwoPlayerCoop => {
                let p1 = self.players.first().map(|p| p.score).unwrap_or(0);
                let p2 = self.players.get(1).map(|p| p.score).unwrap_or(0);
                format!("P1 {p1}   P2 {p2}")
            }
        }
    }
}

/// "LIVES * * *" from a life count (empty tail once a cannon is out).
fn lives_text(lives: u32) -> String {
    format!("LIVES {}", "* ".repeat(lives as usize).trim_end())
}
