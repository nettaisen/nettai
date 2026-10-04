--!strict
-- Sparkles (the original's effect object #0x11, `sub_80E19BC`): the shared ones (@common/vdoll/sparkles) of BN6's
-- kind and sound. VDoll's curse (./curse) marks the combatants it hits with them. See docs/engine/shot-chips.md §14.2.

local common = require("@common/vdoll/sparkles")

return common.make { id = "bn6:vdoll/sparkles", sound = asset.sound("bn6:hit-bomb-0") }
