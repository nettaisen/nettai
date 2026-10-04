--!strict
-- Firing the projectile, as BN5 and BN6 share it (@exelib/projectile/projectile,
-- the original's attack object #0), of a game's kind and buster
-- (`projectile.make`): the shot of the buster and the charged shot, the cannons, AirShot,
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

local shot = require("./projectile/projectile")

local projectile = {}

-- A game's firing: its projectile kind and its buster.
function projectile.make(kind: Kind, buster: Buster): Projectiles
    local p = {}

    -- `sub_80C4FFE`: a projectile from `owner` on panel (x, y), with the
    -- owner's side and flip; nil if the attack pool is full.
    function p.spawn(owner: Object, x: number, y: number, s: ProjectileShot): Object?
        return shot.spawn(kind, owner, x, y, s)
    end

    -- `sub_800FAAC`: a projectile from the panel in front of `navi`; returns
    -- the buster's recovery counted from that panel (`sub_800FAF6`: ticks by
    -- the navi's Rapid and the open panels ahead, @exelib/buster).
    function p.fire(navi: Object, s: ProjectileShot): number
        local x, y = navi.panel_x + navi.facing, navi.panel_y
        shot.spawn(kind, navi, x, y, s)
        return buster.recovery(navi, x, y)
    end

    return p
end

return projectile
