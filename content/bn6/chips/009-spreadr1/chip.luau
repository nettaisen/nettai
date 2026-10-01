--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record or weapon that names the original's action 0x25 runs
-- this module, which runs the action it means (chips/spreadr), picked by
-- the subtype (the gun and the bullet's height, `dword_80ECC54`,
-- `dword_80ECC5C`) and the attack's first parameter (the bullet's row,
-- objects/bullet `by_number`). Records reach it where something names a
-- chip by number: the Spreaders (H-Burst's Program Advance), and the
-- weapons that set up action 0x25 by number: SpoutCross's charged shot
-- (0x0C) and three link navis' (0x5F, 0x74, 0x80, not ported yet). It goes
-- when those are definitions (step 5, phase C).

local bullet = require("../../objects/bullet/bullet")
local spreader = require("../spreadr/action")
local spreaders = require("../spreadr/chips")

local legacy = {
    state = spreader.STATE,
}

-- The gun and height by subtype.
local GUNS: { [number]: { gun: AttachmentLook, z: number } } = {
    [0] = { gun = spreader.guns.spreader, z = 0x10_0000 },
    [1] = { gun = spreader.guns.spreader, z = 0x10_0000 },
    [2] = { gun = spreader.guns.water_cannon, z = 0 },
    [3] = { gun = spreader.guns.spreader, z = 0x10_0000 },
}

-- The actions the numbered users mean, by subtype and bullet row: the
-- Spreaders' and SpoutCross's are the chips'; the link navis' weapons'
-- are made here until those weapons are definitions.
local function link(subtype: number, row: number): Action
    local g = GUNS[subtype]
    return spreader.action { gun = g.gun, z = g.z, bullet = bullet.by_number(row) }
end
local BY_USE: { [number]: { [number]: Action } } = {
    [0] = { [3] = spreaders.spreadr1 },
    [1] = { [8] = link(1, 8) },
    [2] = { [0xD] = spreaders.spout_charge, [7] = link(2, 7), [4] = link(2, 4) },
}

function legacy.update(me: Object, s: any)
    local v, row = me.variant, me:attack_param(1)
    local uses = BY_USE[v]
    local a = if uses then uses[row] else nil
    if not a then
        error(string.format("spreader subtype %#x with bullet row %#x: nothing sets it up", v, row))
    end
    a.update(me, s)
end

return legacy
