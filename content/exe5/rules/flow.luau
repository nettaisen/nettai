--!strict
-- EXE5's battle flow where it differs from EXE6's (docs/design/exe5-map.md §15.3 items 7 and 13, §15.4), the arena's:
-- a custom screen's result is 49 words on the link (0x08009A5E: its NaviStats 0x60 bytes, EXE6's 0x64), so it is in
-- a tick sooner; its Team Battle custom screen (0x08025EF2, run by the custom screen mode 0x08007F50 in place of
-- EXE6's `sub_8026A28`) takes it as EXE6's does (0x080266FA, a tick after both results are in its state 8 closes it,
-- 0x08025FEC); after the reversions it opens the custom screen
-- straight away (no state 0x24); no escape check (EXE6's `sub_800AAD6` is absent); the result's banner holds at
-- least 102 ticks, 65 in a special battle. And what an EXE5 console shows otherwise (presentation, which the frontend
-- reads of a console's own game): its intro fades in from black (0x080E0698 tests a byte the open bus gives).

return {
    result_words = 49,
    sequencer_before_custom = false,
    escape_check = false,
    result_wait = { normal = 0x66, special = 65 },
    intro_from_black = true,
    -- No low-HP music: EXE5's ROM has no tempo or pitch control (EXE6's `sub_8009158` switch is absent).
    low_hp_music = false,
    -- A win shows the winner's navi's banner only in a link battle that is an operation battle (0x080074D2: battle
    -- flag 0x40, 0x080090E8, then 0x080090C0's table); a Team Battle's and a NetBattle's shows "ENEMY DELETED", or
    -- "YOU WIN" on the judge's ruling (the roles `win` and `win_judged`). EXE6's link battles all show the navi's.
    navi_win_banner = "operation_battle",
    -- The intro returns after its init, as EXE6's.
    intro_steps_on_init = false,
    -- L or R with a full gauge: the navi's input decode asks for the custom screen, which the fight reads the next
    -- tick, as EXE6's.
    custom_request = "navi_input",
    -- A turn starts with the transformation sequencer (the souls'), as EXE6's.
    sequencer_at_turn_start = true,
}
