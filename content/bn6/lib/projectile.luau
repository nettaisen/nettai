--!strict
-- Firing BN6's projectile (objects/projectile, the original's attack object
-- #0; @exelib/projectile): `projectile.fire` from the panel in front of its
-- navi as the buster does, or `projectile.spawn`, from any panel.

local buster = require("./buster")
local common = require("@exelib/projectile")
local shot = require("../objects/projectile/projectile")

return common.make(shot.kind, buster)
