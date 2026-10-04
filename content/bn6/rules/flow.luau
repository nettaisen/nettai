--!strict
-- BN6's battle flow where games differ (docs/design/bn5-map.md §15.3 items 7 and 13), the arena's: a custom
-- screen's result is 50 words on the link (`sub_800B3A2`, the sending side's game's), and the screen closes on the
-- tick after both results are in (`sub_8026A28`); before it opens, the transformation
-- sequencer runs once more after the reversions (state 0x24, `sub_8008492`); the fight checks for an escape
-- (`sub_800AAD6`); the result's banner holds at least 102 ticks, 94 in a special battle and for a win in battle
-- modes 4, 5 and 8 (`sub_80081A4`, `sub_800825A`).

return define.rules("bn6:flow", {
    result_words = 50,
    sequencer_before_custom = true,
    escape_check = true,
    result_wait = { normal = 0x66, special = 0x5E },
})
