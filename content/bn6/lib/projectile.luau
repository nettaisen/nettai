--!strict
-- Firing the projectile (objects/projectile, the original's attack object
-- #0): the shot of the buster and the charged shot, the cannons, AirShot,
-- Reflectr and more. An action fires one with `projectile.fire`, from the
-- panel in front of its navi as the buster does, or `projectile.spawn`,
-- from any panel. What a shot does is its variant (a record: the shared
-- ones in objects/projectile/variants.luau, a chip's own with the chip):
-- its collision, element, spark, look, status and bug, what it does to the
-- panel it hits, whether it bursts. See docs/engine/objects-and-player.md
-- §B8.
--
--     local projectile = require("../../lib/projectile")
--     local variants = require("../../objects/projectile/variants")
--     -- A cannon shot 24 pixels up with the attack's damage word; the
--     -- buster's recovery from the panel in front.
--     s.recovery = projectile.fire(me, { variant = variants.cannon, damage = damage, z = 0x18_0000 })
--     -- A shot from (x, y) that also moves a row down on its first step.
--     projectile.spawn(me, x, y, { variant = v, damage = damage, z = 0x18_0000, row = 1 })
--
-- A shot appears on its panel and hits it on the tick it is fired (it
-- runs right after its spawner), then steps a panel every two ticks.

local buster = require("./buster")
local shot = require("../objects/projectile/projectile")

local projectile = {}

-- `sub_80C4FFE`: a projectile from `owner` on panel (x, y), with the
-- owner's side and flip; nil if the attack pool is full.
function projectile.spawn(owner: Object, x: number, y: number, s: ProjectileShot): Object?
    return shot.spawn(owner, x, y, s)
end

-- `sub_800FAAC`: a projectile from the panel in front of `navi`; returns
-- the buster's recovery counted from that panel (`sub_800FAF6`: ticks by
-- the navi's Rapid and the open panels ahead, lib/buster.luau).
function projectile.fire(navi: Object, s: ProjectileShot): number
    local x, y = navi.panel_x + navi.facing, navi.panel_y
    shot.spawn(navi, x, y, s)
    return buster.recovery(navi, x, y)
end

return projectile
