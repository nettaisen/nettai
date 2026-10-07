--!strict
-- What an EXE4 navi's stats hold when they are made fresh, beyond what the navi's own row gives it (its `fresh` and
-- its `weapons`): what a new NaviStats block holds (0x0800D6BE, docs/design/exe4-map.md §3.3). The Regular memory is
-- 4 (a new game's: the save's 0x2148 is the Regular memory less 4), the custom level 5 and the mood 0x99.

return {
    reg_up = 4,
    custom_level = 5,
    mood = 0x99,
}
