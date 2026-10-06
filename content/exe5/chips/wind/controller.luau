--!strict
-- EXE5's Wind and Fan controller (effect object #0x25, 0x080E329C): the shared one (@exelib/wind/controller, EXE6's
-- code) placing EXE5's fan (./fan). Its position is the dimming handler's registers: the user's row, the element and
-- the hook's own address (0x080E32E4, the handler's `bx r3`, Thumb).
-- (The address is Team ProtoMan US's ROM's; compat/games.toml has the other ROMs'.)

local common = require("@exelib/wind/controller")
local fan = require("./fan")

local HOOK_ADDRESS = 0x080E32E5

local made = common.make {
    id = "wind/controller",
    spawn_fan = fan.spawn,
    position = function(user: Object, spec: DimmingChipSpec): Vec3
        return Vec3.new(user.panel_y, spec.element, HOOK_ADDRESS)
    end,
}

-- 0x080E32E4: a chip's `dimming` hook, the controller placing a fan of `variant` (the record's first parameter).
return { hook = made.hook }
