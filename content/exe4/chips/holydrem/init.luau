--!strict
-- HolyDrem (EXE4 chip 0x133, Red Sun's Giga chip): the dimming chips' variant 0x2C: in the dimming, a stand-in of the
-- user fires a shot along the row, then one more for each holy panel on the field, each panel turned Normal. Its
-- controller (effect object #0x59, 0x080E8418: @exelib/holydrem/controller, EXE5's code) sends out EXE4's stand-in
-- (./stand_in, attack object #0x82) with the damage plus the bonus; its hook (0x080E8510) takes the user's side alone.
-- Its shot is ./shot.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local common = require("@exelib/holydrem/controller")
local stand_in = require("@self/stand_in")

local CONTROLLER = common.make { id = "holydrem/controller", stand_in = stand_in.spawn, side_only = true }

local chip: Chip = {
    codes = { "H" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 92,
    damage = 50,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = CONTROLLER.hook,
    megaman = "light",
}

return { holydrem = chip }
