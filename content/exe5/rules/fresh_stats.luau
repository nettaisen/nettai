--!strict
-- What an EXE5 navi's stats hold when they are made fresh, beyond what the navi's own row gives it (its `fresh`
-- and its `weapons`: 0x0801D55F): what 0x08010C00 writes for every navi, MegaMan and the team navis alike, as
-- 0x080111AA makes each of a new game's seven save blocks (0x08010BB4). The Regular memory is 4 and the custom
-- level 5; the mood is 0x99 (a save carries its own from battle to battle, and a round's start sets it by the
-- light/dark value). The halfword at +0x44 is the light/dark value, 500: the light and dark system's
-- (rules/light_dark's `karma`). A team navi's HP is its story's (the navi's `story`).

return {
    reg_up = 4,
    custom_level = 5,
    mood = 0x99,
}
