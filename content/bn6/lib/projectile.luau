--!strict
-- Firing the projectile (attack object #0, objects/projectile): the shot
-- of the buster and the charged shot, the cannons, AirShot, Reflectr and
-- many more. An action fires one with `projectile.fire`, from the panel in
-- front of its navi as the buster does, or `projectile.spawn`, from any
-- panel. What a shot does is its kind's data (data.objects.projectiles,
-- by the kind number the game's routine passes as Param1): its collision,
-- element, spark, look, status and bug, what it does to the panel it hits,
-- whether it bursts. See docs/engine/objects-and-player.md §B8.
--
--     local projectile = require("../../lib/projectile")
--     -- A plain shot (kind 0) 24 pixels up, with the attack's damage and
--     -- counter byte; the buster's recovery from the panel in front.
--     local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
--     s.recovery = projectile.fire(me, { kind = 0, damage = damage, z = 0x18_0000 })
--     -- A shot from (x, y) that also moves a row down on its first step.
--     projectile.spawn(me, x, y, { kind = k, damage = damage, z = 0x18_0000, row = 1 })
--
-- A shot appears on its panel and hits it on the tick it is fired (it
-- runs right after its spawner), then steps a panel every two ticks.

local buster = require("./buster")
local shot = require("../objects/projectile/projectile")

local projectile = {}

-- What a projectile is fired with, `ProjectileShot` (types.d.luau): its
-- `kind` (data.objects.projectiles), the `damage` word (damage | counter
-- byte << 16), its height `z` (16.16) and the `row` it also moves on its
-- first step (1 down, -1 up; none by default).

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
