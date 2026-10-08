--!strict
-- EXE4's Wind and Fan controller (effect object #0x25, 0x080E4AD4; its effect 0x080E4B18): the shared one
-- (@exelib/wind/controller, EXE6's code) placing EXE4's fan (./fan) on the panel in front of the user if it is free
-- solid ground; its spawn (0x080E4B60) takes the user's side, not its facing. (Its position is the dimming handler's
-- registers, which exe4-compat's kinds.toml skips.)

local common = require("@exelib/wind/controller")
local fan = require("./fan")

local made = common.make {
    id = "wind/controller",
    spawn_fan = fan.spawn,
    side_only = true,
}

-- 0x080E4B60: a chip's `dimming` hook, the controller placing a fan of `variant` (the record's first parameter).
return { hook = made.hook }
