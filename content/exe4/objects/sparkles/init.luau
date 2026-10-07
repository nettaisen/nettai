--!strict
-- EXE4's sparkles (effect object 0x11, 0x080E3328: the shared ones, @exelib/vdoll/sparkles, EXE6's code but for where
-- they show), of EXE4's kind and sound (0x70): they stay where their object was as they started (0x080E3348 keeps its
-- position). SeekBom's bomb bursts in them (chips/seekbom).

local common = require("@exelib/vdoll/sparkles")

return common.make { id = "sparkles", sound = asset.sound("hit-bomb-1"), stays = true }
