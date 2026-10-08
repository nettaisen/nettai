--!strict
-- Firing EXE4's projectile (objects/projectile): `projectile.fire` from the panel in front of its navi, the
-- buster's recovery from there (0x0800B590, EXE6's `sub_800FAAC` but for its constants), or `projectile.spawn`,
-- from any panel.

local common = require("@exelib/projectile/projectile")
local recovery_rows = require("../rules/buster").recovery
local shot = require("../objects/projectile")

local projectile = {}

-- What stops the recovery's count of open panels (0x0800B6DC: one mask whichever side shoots, EXE5's).
local CLOSED = 0x0F880080

-- 0x0800B6DC: the ticks of recovery by a Rapid (the recovery table's row: the buster's the shooter's Rapid, other
-- shots' their own) and the open panels ahead of (x, y), counted on from it (5 at most).
function projectile.recovery(navi: Object, x: number, y: number, rapid: number): number
    local open = 0
    while true do
        local f = field.flags(x, y)
        if f == 0 or bit32.btest(f, CLOSED) then
            break
        end
        open += 1
        x += navi.facing
    end
    local row = recovery_rows[rapid + 1]
    if row == nil then
        error(string.format("Rapid %d reads past the buster's recovery table", rapid))
    end
    return row[math.min(open, 5) + 1]
end

-- 0x080CD45A: a projectile from `owner` on panel (x, y), with the owner's side and flip; nil if the attack pool is
-- full.
function projectile.spawn(owner: Object, x: number, y: number, s: ProjectileShot): Object?
    return common.spawn(shot.kind, owner, x, y, s)
end

-- 0x0800B590: a projectile from the panel in front of `navi`; the buster's recovery counted from that panel.
function projectile.fire(navi: Object, s: ProjectileShot): number
    local x, y = navi.panel_x + navi.facing, navi.panel_y
    common.spawn(shot.kind, navi, x, y, s)
    return projectile.recovery(navi, x, y, battle.navi(navi.alliance).rapid)
end

return projectile
