--!strict
-- How EXE5's chips run (docs/design/exe5-map.md §14.3, §15.3 items 10 and 11), read from the section of the
-- chip's own root: the dimming handler (action 0x15, 0x080EC318) spawns the dimming and leaves the action on
-- the same frame, as the instant chips' action does (0x080EC6F6), so the user stands during the dimming;
-- AntiNavi's sparkle shows 16 pixels up from the navi chip's panel's center (0x080093A2); a hand's chip carries what
-- MeddySoul's capsules mixed into it (its entry's marks).

return {
    leave_on_use = true,
    anti_navi_sparkle = { dy = 0, z = 16 },
    -- What a hand entry's marks add to the damage word at the chip's use, by the mark's bit (0x08010368): 0x02 its
    -- 0x4000 (the white capsule: paralysis), 0x04 its 0x2000 (the yellow: confusion), 0x08 its 0x1000 (the black:
    -- blindness), 0x20 its 0x0800 (the purple: the HP bug). And 0x10 (the pink) heals the user a tenth of its HP at
    -- the use (0x0800FFF6).
    mark_damage = { 0, 0x4000, 0x2000, 0x1000, 0, 0x0800, 0, 0 },
    mark_heals = 0x10,
    -- Beat takes any Mega or Giga chip, dark or not.
    beat_spares_dark = false,
    -- A navi's idle, its attacks and a dimming's start hide the chip window on the navi's own console only
    -- (0x080F02E6 on, 0x08009FBA: EXE6's `sub_80F0354` and `object_timefreezeBegin`).
    window_hides_every_console = false,
    -- A charged chip is used with its charge, whatever its family (0x0800D9CC).
    charged_null_alt_routine = false,
    -- A use the rules refuse (rules/light_dark) becomes the invalid chip, the attack's chip too (0x080100F6 stores
    -- 0x185).
    refused_keeps_chip = false,
}
