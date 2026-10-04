--!strict
-- Invisibl's dimming controller (the original's effect object #0x5D,
-- `sub_80E74D4`), which the second WhiCapsl (chips/whicapsl-invisible)
-- uses too: the shared controller (@exelib/invisibl/controller), BN6's kind
-- and sound. A chip's `dimming` hook is `invisible.hook(ticks)`.

local common = require("@exelib/invisibl/controller")

return common.make { id = "bn6:invisibl/controller", sound = asset.sound("bn6:invisible") }
