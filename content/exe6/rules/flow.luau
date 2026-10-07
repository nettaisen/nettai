--!strict
-- EXE6's battle flow where games differ (docs/design/exe5-map.md §15.3 items 7 and 13), the arena's: a custom
-- screen's result is 50 words on the link (`sub_800B3A2`, the sending side's game's), and the screen closes on the
-- tick after both results are in (`sub_8026A28`); before it opens, the transformation
-- sequencer runs once more after the reversions (state 0x24, `sub_8008492`); the fight checks for an escape
-- (`sub_800AAD6`); the result's banner holds at least 102 ticks, 94 in a special battle and for a win in battle
-- modes 4, 5 and 8 (`sub_80081A4`, `sub_800825A`); a link battle's win shows the winner's navi's banner
-- (`sub_80081A4`: `sub_800A8D4`'s table, whatever the result), any other "ENEMY DELETED", or "YOU WIN" on the judge's
-- ruling (the roles `win` and `win_judged`).

return {
    result_words = 50,
    sequencer_before_custom = true,
    escape_check = true,
    result_wait = { normal = 0x66, special = 0x5E },
    -- What an EXE6 console shows (presentation, which the frontend reads of a console's own game): a set's first
    -- battle's intro fades in from white.
    intro_from_black = false,
    -- The low-HP music switch (`sub_8009158`): while a console's navi is at a quarter of its HP or less, the music
    -- plays a semitone higher and 282/256 as fast.
    low_hp_music = true,
    navi_win_banner = "link_battle",
    -- The intro (`sub_80091F0`) returns after its init.
    intro_steps_on_init = false,
    -- A turn starts with the transformation sequencer, run twice (fighting state 0, `sub_800840C`); its banner after.
    sequencer_at_turn_start = true,
    -- L or R with a full gauge: the navi's input decode asks for the custom screen (`sub_8012FC8` sets battle flag
    -- 0x10), which the fight reads the next tick (`sub_800A1D0`).
    custom_request = "navi_input",
}
