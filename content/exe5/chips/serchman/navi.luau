--!strict
-- SearchMan, the navi of EXE5's SearchMan chips (EXE5's actor object #0x31, 0x080C11D8): @exelib/serchman/navi with
-- EXE5's look: his sprite, his sounds, his rifle up in animation 16 and a shot in 17, his scope (./scope) and shot
-- (./shot). Each chip summons him with its palette (0x0800DA8A by the chip's level: 0x0801D737's navi 3 count by
-- 0x0800DAE8's row: SP's 2, DS's 3) and its scope's sweep (0x080E5344 by the level: 6 ticks a panel, SP's and DS's 4).

local common = require("@exelib/serchman/navi")
local scope = require("./scope")
local shot = require("./shot")

local searchman = common.make {
    id = "serchman/navi",
    sprite = asset.sprite("searchman"),
    appear_sound = asset.sound("appear"),
    shot_sound = asset.sound("col-army-2"),
    aim_anim = 0x10,
    shot_anim = 0x11,
    scope = scope,
    shot = shot,
}

-- The summon table's entry 2: SearchMan of a chip's palette and sweep (`summon { palette = ..., sweep = ... }`).
return { kind = searchman.kind, summon = searchman.summon }
