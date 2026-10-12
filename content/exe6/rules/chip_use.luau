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
    -- A navi's idle, its attacks and a dimming's start hide the chip window on the navi's own console only
    -- (`sub_80F0354`, `object_timefreezeBegin`: `battle_networkInvert`).
    window_hides_every_console = false,
    -- A charged Null chip runs the alternative A-charge routine, the attack's chip cleared (`sub_800FB54`).
    charged_null_alt_routine = true,
    -- A chip the rules put in a use's place is the attack's chip (no rule of EXE6's refuses a use: its dark chips'
    -- substitute comes before the record is loaded, `sub_8010D58`).
    refused_keeps_chip = false,
}
