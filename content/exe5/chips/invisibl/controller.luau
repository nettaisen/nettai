--!strict
-- EXE5's Invisibl's dimming controller (effect object #0x5D, EXE6's code: @exelib/invisibl/controller), EXE5's kind;
-- its sound EXE6's number.

local common = require("@exelib/invisibl/controller")

return common.make { id = "invisibl/controller", sound = asset.sound("invisible") }
