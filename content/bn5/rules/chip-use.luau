--!strict
-- How BN5's chips run (docs/design/bn5-map.md §14.3, §15.3 items 10 and 11), read from the section of the
-- chip's own root: the dimming handler (action 0x15, 0x080EC318) spawns the dimming and leaves the action on
-- the same frame, as the instant chips' action does (0x080EC6F6), so the user stands during the dimming;
-- AntiNavi's sparkle shows 16 pixels up from the navi chip's panel's center (0x080093A2).

return {
    leave_on_use = true,
    anti_navi_sparkle = { dy = 0, z = 16 },
}
