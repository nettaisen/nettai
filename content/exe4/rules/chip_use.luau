--!strict
-- How EXE4's chips run (docs/design/exe4-map.md §15 chip_use), read from the section of the chip's own root: the
-- dimming handler (action 0x0C, 0x080EB9EA: EXE5's code) spawns the dimming and leaves the action on the same frame,
-- so the user stands during the dimming; AntiNavi's sparkle shows 16 pixels up from the navi chip's panel's center
-- (0x0800815C, EXE5's); a hand's chip carries no mixed modifiers (EXE4 has no capsules).

return {
    leave_on_use = true,
    anti_navi_sparkle = { dy = 0, z = 16 },
    mixed_modifiers = false,
    -- Beat lets a dark Mega or Giga chip go (0x0800C86C tests its flag 0x20).
    beat_spares_dark = true,
    -- A dimming's start hides the chip window (HUD task 0x10) on every console, whoever started it (0x08008BD8: no
    -- check of the starter's side); the idle shows it again after.
    dimming_hides_every_window = true,
}
