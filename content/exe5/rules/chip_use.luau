--!strict
-- How EXE5's chips run (docs/design/exe5-map.md §14.3, §15.3 items 10 and 11), read from the section of the
-- chip's own root: the dimming handler (action 0x15, 0x080EC318) spawns the dimming and leaves the action on
-- the same frame, as the instant chips' action does (0x080EC6F6), so the user stands during the dimming;
-- AntiNavi's sparkle shows 16 pixels up from the navi chip's panel's center (0x080093A2); a hand's chip carries what
-- MeddySoul's capsules mixed into it (0x08010368: the modifier bits 0x08 and 0x20 the damage word's blindness and HP
-- bug, beside EXE6's two; 0x0800FFF6: 0x10 a tenth of the user's HP healed).

return {
    leave_on_use = true,
    anti_navi_sparkle = { dy = 0, z = 16 },
    mixed_modifiers = true,
    -- Beat takes any Mega or Giga chip, dark or not.
    beat_spares_dark = false,
    -- A navi's idle, its attacks and a dimming's start hide the chip window on the navi's own console only
    -- (0x080F02E6 on, 0x08009FBA: EXE6's `sub_80F0354` and `object_timefreezeBegin`).
    window_hides_every_console = false,
    -- A charged chip is used with its charge, whatever its family (0x0800D9CC).
    charged_null_alt_routine = false,
}
