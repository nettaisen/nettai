--!strict
-- How BN6's chips run (docs/design/bn5-map.md §15.3 items 10 and 11), read from the section of the chip's own
-- root (docs/design/rules-in-luau.md §7.5): a dimming chip's user waits in its action until the dimming ends
-- (`sub_80EBD9C`), and a weapon's own effect 8 more ticks (`sub_80EC39C`); AntiNavi's sparkle shows 16 pixels
-- down the field and 32 up from the navi chip's panel's center (`sub_800ABC6`).

return define.rules("bn6:chip-use", {
    leave_on_use = false,
    anti_navi_sparkle = { dy = 16, z = 32 },
})
