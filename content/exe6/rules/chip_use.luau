--!strict
-- How EXE6's chips run (docs/design/exe5-map.md §15.3 items 10 and 11), read from the section of the chip's own
-- root (docs/design/rules-in-luau.md §7.5): a dimming chip's user waits in its action until the dimming ends
-- (`sub_80EBD9C`), and a weapon's own effect 8 more ticks (`sub_80EC39C`); AntiNavi's sparkle shows 16 pixels
-- down the field and 32 up from the navi chip's panel's center (`sub_800ABC6`).

return {
    leave_on_use = false,
    anti_navi_sparkle = { dy = 16, z = 32 },
    -- A chip's use knows the hand's modifier bits 0x02 and 0x04 alone (`sub_8012C34`): none of EXE5's capsules'.
    mixed_modifiers = false,
    -- Beat takes any Mega or Giga chip, dark or not.
    beat_spares_dark = false,
    -- A dimming's start hides the chip window on its starter's console only (`object_timefreezeBegin`).
    dimming_hides_every_window = false,
}
