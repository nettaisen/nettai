--!strict
-- Firing EXE5's projectile (objects/projectile; @exelib/projectile): `projectile.fire` from the panel in front of its
-- navi as the buster does (EXE5's buster's recovery), or `projectile.spawn`, from any panel.

local buster = require("./buster")
local common = require("@exelib/projectile")
local shot = require("../objects/projectile")

return common.make(shot.kind, buster)
