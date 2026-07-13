//! Space Invaders achievement definitions and unlock logic.
//!
//! Registered once in `init()`. Clear/perfect achievements unlock from the
//! win path in `gameplay::flow`; the sharpshooter streak unlocks live in
//! `gameplay::combat` the moment the streak hits `SHARPSHOOTER_TARGET`.

use engine_core::prelude::*;

use crate::constants::STARTING_LIVES;
use crate::types::SpaceInvadersGame;

/// IDs — kept as `&'static str` so the compiler catches typos at call sites.
pub(crate) const CLEAR_NORMAL:     &str = "clear_normal";
pub(crate) const CLEAR_INSANE:     &str = "clear_insane";
pub(crate) const CLEAR_RIDICULOUS: &str = "clear_ridiculous";
pub(crate) const CLEAR_INSICULOUS: &str = "clear_insiculous";

pub(crate) const PERFECT_NORMAL:     &str = "perfect_normal";
pub(crate) const PERFECT_INSANE:     &str = "perfect_insane";
pub(crate) const PERFECT_RIDICULOUS: &str = "perfect_ridiculous";
pub(crate) const PERFECT_INSICULOUS: &str = "perfect_insiculous";

pub(crate) const SHARPSHOOTER: &str = "sharpshooter";
pub(crate) const LAST_STAND:   &str = "last_stand";
pub(crate) const UFO_HUNTER:   &str = "ufo_hunter";

/// Grouped display order for the achievements page. First tuple element is
/// the section header, second is the list of ids to render under it.
pub(crate) const DISPLAY_SECTIONS: &[(&str, &[&str])] = &[
    ("Repelled Invasions",
        &[CLEAR_NORMAL, CLEAR_INSANE, CLEAR_RIDICULOUS, CLEAR_INSICULOUS]),
    ("Perfect Defenses",
        &[PERFECT_NORMAL, PERFECT_INSANE, PERFECT_RIDICULOUS, PERFECT_INSICULOUS]),
    ("Skill",
        &[SHARPSHOOTER, LAST_STAND, UFO_HUNTER]),
];

/// Register every Space Invaders achievement. Call once from `Game::init`.
pub(crate) fn register_all(mgr: &mut AchievementManager) {
    mgr.register(Achievement::new(CLEAR_NORMAL,
        "Earth Defender",
        "Repel the invasion in Normal mode."));
    mgr.register(Achievement::new(CLEAR_INSANE,
        "Blitz Breaker",
        "Repel the invasion in Insane mode."));
    mgr.register(Achievement::new(CLEAR_RIDICULOUS,
        "Twin Cannon Commander",
        "Repel the invasion in Ridiculous mode."));
    mgr.register(Achievement::new(CLEAR_INSICULOUS,
        "Insiculous Overkill",
        "Repel the invasion in Insiculous mode."));

    mgr.register(Achievement::new(PERFECT_NORMAL,
        "Untouchable",
        "Clear Normal mode without losing a life."));
    mgr.register(Achievement::new(PERFECT_INSANE,
        "Untouchable Under Fire",
        "Clear Insane mode without losing a life."));
    mgr.register(Achievement::new(PERFECT_RIDICULOUS,
        "Untouchable Overdrive",
        "Clear Ridiculous mode without losing a life."));
    mgr.register(Achievement::new(PERFECT_INSICULOUS,
        "Insiculously Untouchable",
        "Clear Insiculous mode without losing a life."));

    mgr.register(Achievement::new(SHARPSHOOTER,
        "Sharpshooter",
        "Land 10 kill shots in a row without wasting a bullet."));
    mgr.register(Achievement::new(LAST_STAND,
        "Last Stand",
        "Repel the invasion on your very last life."));
    mgr.register(Achievement::new(UFO_HUNTER,
        "UFO Hunter",
        "Shoot down a mystery ship."));
}

impl SpaceInvadersGame {
    /// Called from the win path when the last invader falls.
    pub(crate) fn unlock_win_achievements(&self, ctx: &mut GameContext) {
        ctx.achievements.unlock(chaos_clear_id(self.chaos_mode));
        if self.lives == STARTING_LIVES {
            ctx.achievements.unlock(chaos_perfect_id(self.chaos_mode));
        }
        if self.lives == 1 {
            ctx.achievements.unlock(LAST_STAND);
        }
        // SHARPSHOOTER (gameplay::combat) and UFO_HUNTER (gameplay::ufo)
        // unlock live, not here.
    }
}

fn chaos_clear_id(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal     => CLEAR_NORMAL,
        ChaosMode::Insane     => CLEAR_INSANE,
        ChaosMode::Ridiculous => CLEAR_RIDICULOUS,
        ChaosMode::Insiculous => CLEAR_INSICULOUS,
    }
}

fn chaos_perfect_id(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal     => PERFECT_NORMAL,
        ChaosMode::Insane     => PERFECT_INSANE,
        ChaosMode::Ridiculous => PERFECT_RIDICULOUS,
        ChaosMode::Insiculous => PERFECT_INSICULOUS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_all_adds_eleven() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        assert_eq!(mgr.total(), 11);
    }

    #[test]
    fn chaos_clear_id_maps_each_mode() {
        assert_eq!(chaos_clear_id(ChaosMode::Normal),     CLEAR_NORMAL);
        assert_eq!(chaos_clear_id(ChaosMode::Insane),     CLEAR_INSANE);
        assert_eq!(chaos_clear_id(ChaosMode::Ridiculous), CLEAR_RIDICULOUS);
        assert_eq!(chaos_clear_id(ChaosMode::Insiculous), CLEAR_INSICULOUS);
    }

    #[test]
    fn chaos_perfect_id_maps_each_mode() {
        assert_eq!(chaos_perfect_id(ChaosMode::Normal),     PERFECT_NORMAL);
        assert_eq!(chaos_perfect_id(ChaosMode::Insane),     PERFECT_INSANE);
        assert_eq!(chaos_perfect_id(ChaosMode::Ridiculous), PERFECT_RIDICULOUS);
        assert_eq!(chaos_perfect_id(ChaosMode::Insiculous), PERFECT_INSICULOUS);
    }

    #[test]
    fn display_sections_cover_every_registered_achievement() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);

        let shown: std::collections::HashSet<&str> = DISPLAY_SECTIONS
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();

        for ach in mgr.all() {
            assert!(
                shown.contains(ach.id.as_str()),
                "{} registered but not in DISPLAY_SECTIONS",
                ach.id
            );
        }
        assert_eq!(shown.len(), mgr.total(), "DISPLAY_SECTIONS has duplicates or extras");
    }

    #[test]
    fn every_id_is_registered() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        for id in [
            CLEAR_NORMAL, CLEAR_INSANE, CLEAR_RIDICULOUS, CLEAR_INSICULOUS,
            PERFECT_NORMAL, PERFECT_INSANE, PERFECT_RIDICULOUS, PERFECT_INSICULOUS,
            SHARPSHOOTER, LAST_STAND, UFO_HUNTER,
        ] {
            assert!(mgr.get(id).is_some(), "{} not registered", id);
        }
    }
}
