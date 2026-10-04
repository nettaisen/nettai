--!strict
-- BN5's sparkles (effect object #0x11, 0x080E1A68: the shared ones, @exelib/vdoll/sparkles, BN6's code) of BN5's
-- kind and sound (0x6F). BN5's VDoll curse (./curse) marks whoever it hits with them.

local common = require("@exelib/vdoll/sparkles")

return common.make { id = "bn5:vdoll/sparkles", sound = asset.sound("bn5:hit-bomb-0") }
