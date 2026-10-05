--!strict
-- What an EXE6 navi's stats hold when they are made fresh, beyond what the navi's own row gives it (its `fresh`
-- and its `weapons`: `byte_80210DD`): what `initNaviStats_WithDefaultStatsMaybe_8013438` writes for every navi, as
-- `init_8013B64` makes a link navi's block at a navi switch and a new game makes each save block. The Regular memory
-- is 4 and the custom level 5; the mood is 0x99 (a round's start sets its own); the Beast Out counter is 3 turns.
-- The block is zeroed first, and the zero at +0x44, the weapon of the A button in battle mode 9, names weapon
-- routine 0: MegaMan's buster, whatever the navi. (The version byte, +0x20, is the console's own, 0 in a Gregar ROM
-- and 1 in a Falzar one: the side's, which a battle's start sets.)

local buster = require("../navis/megaman/weapons/buster")

return {
    reg_up = 4,
    custom_level = 5,
    mood = 0x99,
    beast_out_counter = 3,
    mode9_a = buster.weapon,
}
