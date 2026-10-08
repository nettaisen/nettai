--!strict
-- Blinder (EXE5 chip 0x081): a flash that blinds every enemy navi: the dimming handler's subtype 8 (0x080E3140),
-- whose controller (effect object #0x23, 0x080E3050) is EXE6's code that no EXE6 chip uses (@exelib/blinder/controller,
-- `sub_80E2E34`): the user flashes white for 10 ticks, then a screen flash, the blinding hit over the other side's area
-- (EXE5's row 0x34, blindness for 480 ticks) and 60 more ticks of the user flashing.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local controller = require("@exelib/blinder/controller")
local statuses = require("../rules/status")

-- Its controller (effect object #0x23, 0x080E3050) in EXE5's look: its sound, the blinding hit's row 0x34 at navis,
-- blindness for 480 ticks.
local HOOK = controller.make {
    id = "blinder/controller",
    sound = asset.sound("blinding-flash"),
    is = collision.collision_34,
    hits = collision.hits_navis,
    status = statuses.blind_480,
}

local chip: Chip = {
    codes = { "A", "D", "H", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 1,
    mb = 9,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = HOOK,
    auto_battle = { class = "at_once" },
}

return { blinder = chip }
