--!strict
-- EXE5's sparkles (effect object #0x11, 0x080E1A68: the shared ones, @exelib/vdoll/sparkles, EXE6's code) of EXE5's
-- kind and sound (0x6F). EXE5's VDoll curse (./curse) marks whoever it hits with them.

local common = require("@exelib/vdoll/sparkles")

return common.make { id = "vdoll/sparkles", sound = asset.sound("hit-bomb-0") }
