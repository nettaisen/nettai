--!strict
-- EXE4's Invis's dimming controller (effect object #0x5D, 0x080E8A68: EXE6's Invisibl's code, @exelib/invisibl/
-- controller, its spawner 0x080E8ADA), EXE4's kind; its sound EXE4's number (0x0800C6BC, EXE6's `sub_8010474`, plays
-- 0x109).

local common = require("@exelib/invisibl/controller")

return common.make { id = "invis/controller", sound = asset.sound("invisible") }
