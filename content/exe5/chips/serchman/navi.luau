--!strict
-- SearchMan, the navi of EXE5's SearchMan chips (EXE5's actor object #0x31, 0x080C11D8): @exelib/serchman/navi with
-- EXE5's look: his sprite, his sounds, his palettes by the level (0x0800DA8A: 0x0801D737's navi 3 count by
-- 0x0800DAE8's row: SP's 2, DS's 3), his rifle up in animation 16 and a shot in 17, his scope (./scope) and shot
-- (./shot).

local common = require("@exelib/serchman/navi")
local scope = require("./scope")
local shot = require("./shot")

local searchman = common.make {
    id = "serchman/navi",
    sprite = asset.sprite("searchman"),
    appear_sound = asset.sound("appear"),
    shot_sound = asset.sound("col-army-2"),
    palettes = { [0] = 0, 0, 0, 2, 3 },
    aim_anim = 0x10,
    shot_anim = 0x11,
    scope = scope,
    shot = shot,
}

return {
    kind = searchman.kind,
    -- SearchMan's, SerchMnSP's and SerchMnDS's (their Param1s 0, 3 and 4): the summon table's entry 2.
    summon = searchman.summon_at(0),
    summon_sp = searchman.summon_at(3),
    summon_dark = searchman.summon_at(4),
}
