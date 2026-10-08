--!strict
-- EXE4's north wind (attack object #0x5A, 0x080D8BC8, and its streaks, effect object #0x3A, 0x080E62C0: EXE6's code,
-- @exelib/nrthwind/wind), which WindSoul's change blows (0x080EBADE) and EXE4's NrthWind: its sound as it starts (0x121,
-- the wind's), its streaks' sheet (0c-2e), and its spawns (0x080D8C94, 0x080E6370) giving it its owner's side alone.

local collision = require("../rules/collision")
local north_wind = require("@exelib/nrthwind/wind")

return north_wind.make {
    id = "north-wind",
    sound = asset.sound("wind"),
    streak_sprite = asset.sprite("gust"),
    is = collision.collision_1b,
    hits = collision.nothing,
    copies_flip = false,
}
