--!strict
-- EXE4's battle flow where it differs from EXE6's (docs/design/exe4-map.md §15 flow), the arena's: a custom screen's
-- result is 37 words on the link (0x080088EA: the hand's 0x50 bytes and NaviStats' 0x40); no turn-start sequencer
-- (EXE6's `sub_8008452` and `sub_8008492` are absent: EXE4 has no Cross) and no escape check (`sub_800AAD6`); the
-- result's banner holds at least 102 ticks, 65 in a special battle (0x08007252, as EXE5's). And what an EXE4 console
-- shows otherwise (presentation, which the frontend reads of a console's own game): its intro fades in from white
-- on a set's first battle (0x080E1EBC reads the battle state).
--
-- Not yet (exe4-map.md §18): EXE4's netbattles have no time limit and no double KO.

return {
    result_words = 37,
    sequencer_before_custom = false,
    escape_check = false,
    result_wait = { normal = 102, special = 65 },
    intro_from_black = false,
    -- No low-HP music: EXE6's switch (`sub_8009158`) is absent and EXE4 queues no tempo request (exe4-map.md §3.4).
    low_hp_music = false,
    -- A win shows the winner navi's banner (0x08008534) only under event flag 0x1187 (0x08007252, 0x080406A4), which
    -- the main subsystem 0x080406B0 sets, not a netbattle's: a netbattle's win shows "ENEMY DELETED", or the judge's
    -- banner on the judge's ruling (the roles `win` and `win_judged`; the lab's `flow/buster-duel`, exe4-map.md §16).
    navi_win_banner = "never",
    -- The intro (0x08007464) goes on from its init to its first step (the HUD's setup) on the same tick.
    intro_steps_on_init = true,
    -- L or R with a full gauge: the fight reads both players' keys itself and asks for the custom screen on that tick
    -- (0x08007A2E, from the fighting state, 0x08007188: no reversions).
    custom_request = "joypads",
    -- No transformation sequencer at a turn's start: the fighting machine's state 0 (0x08007064) is the turn's
    -- banner, from the turn's first tick.
    sequencer_at_turn_start = false,
}
