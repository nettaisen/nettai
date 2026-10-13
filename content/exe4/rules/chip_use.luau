--!strict
-- How EXE4's chips run (docs/design/exe4-map.md §15 chip_use), read from the section of the chip's own root: the
-- dimming handler (action 0x0C, 0x080EB9EA: EXE5's code) spawns the dimming and leaves the action on the same frame,
-- so the user stands during the dimming; AntiNavi's sparkle shows 16 pixels up from the navi chip's panel's center
-- (0x0800815C, EXE5's).

return {
    leave_on_use = true,
    anti_navi_sparkle = { dy = 0, z = 16 },
    -- A hand entry's marks add nothing to the damage word and heal no one: EXE4's use (0x0800D4C0) has no such step.
    -- Its marks are 0x01 the Regular chip, 0x02 a chip offered again (JunkSoul's: navis/megaman/forms/junksoul/
    -- recycle) and 0x04 an entry whose hand was installed (rules/dark_chips). What reads them: the install itself;
    -- the use's 0x0800D59A, for which a dark chip offered again doesn't set the side's statistic 2 (the story's
    -- karma reads it, 0x0800F5E2) nor, outside a netbattle, the mood; and the local console's check of its own next
    -- chip, 0x0800865C, which a chip offered again passes without being the folder's (the engine has no such check:
    -- a build is checked before the battle).
    mark_damage = { 0, 0, 0, 0, 0, 0, 0, 0 },
    mark_heals = 0,
    -- Beat lets a dark Mega or Giga chip go (0x0800C86C tests its flag 0x20).
    beat_spares_dark = true,
    -- What hides a navi's chip window (HUD task 0x10) hides it on every console, whichever navi it is (no check of
    -- the side): the idle with its chips not enabled (0x080EEB72), an attack or a chip started (0x080EEBC4 on,
    -- 0x080EED3E), a dimming's start (0x08008BD8). The other player's attack takes the window down until the
    -- navi's own idle shows it again (0x080EEB6C, on its own console).
    window_hides_every_console = true,
    -- A charged chip is used with its charge, whatever its family (0x0800B72C: FireSoul's, soul 5, through its flame,
    -- 0x0800D638, any other's through 0x0800D4B4 with the charge).
    charged_null_alt_routine = false,
    -- A use the rules refuse (rules/light_dark: a chip of the other kind of MegaMan) keeps its chip: only the action
    -- and its variant become the fizzle's (0x0800D5F6: action 0x20, variant 4), so the other console still shows the
    -- chip's name (0x080EECF0) and what follows the use reads the chip's own record (Beat and Rush, RollSoul's heal).
    refused_keeps_chip = true,
}
