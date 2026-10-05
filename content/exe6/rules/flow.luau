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
    -- What an EXE6 console shows otherwise (presentation, which the frontend reads of a console's own game): its
    -- HUD's chip window waits for the navi's first decision (a Japanese console's starts as the custom screen
    -- closes, `sub_8026DC4`: the frontend's, by the console's region), and a set's first battle's intro fades in
    -- from white.
    chip_window_at_close = false,
    intro_from_black = false,
    -- The low-HP music switch (`sub_8009158`): while a console's navi is at a quarter of its HP or less, the music
    -- plays a semitone higher and 282/256 as fast.
    low_hp_music = true,
    navi_win_banner = "link_battle",
    -- The backgrounds a link battle draws one of with its settings (`sub_81209DC`: `byte_8120A20`, 21 entries, in the
    -- table's order; three are there twice, so twice as likely). Whoever draws a match's arena draws from them (the
    -- engine draws none: a round's settings state its background).
    link_backgrounds = {
        asset.background("honeycomb"), asset.background("statues"), asset.background("statues"),
        asset.background("seals"), asset.background("clouds"), asset.background("sprouts"),
        asset.background("calendar-checkers"), asset.background("calendar-mint"), asset.background("calendar-lavender"),
        asset.background("calendar-navy"), asset.background("calendar-blue"), asset.background("calendar-cyan"),
        asset.background("trees"), asset.background("calendar-green"), asset.background("calendar-bright-blue"),
        asset.background("code"), asset.background("globes"), asset.background("code-2"),
        asset.background("code-2"), asset.background("calendar-purple"), asset.background("calendar-purple"),
    },
}
