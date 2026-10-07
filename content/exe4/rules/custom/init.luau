--!strict
-- EXE4's custom screen as its rules answer the framework's hooks (docs/design/exe4-map.md §5): the hand's size.

local custom = {}

-- The hand the screen deals as it opens (0x0801DC8E, its +0x11): the navi's custom level (NaviStats +0x12, 0x0801DCAE)
-- as it is, where EXE6's caps it at 8 and takes its hand-shrink bug from it. (The soul 8's ten, 0x0801DC92, is the
-- souls' to port; a battle type of 64 to 67 deals five, 0x0801DC98, and no netbattle is of one.)
function custom.hand_size(side: number): number
    return battle.navi(side).custom_level
end

return custom
