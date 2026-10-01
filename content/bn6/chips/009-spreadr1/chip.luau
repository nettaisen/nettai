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

local by_number = {
    state = spreader.STATE,
}

-- The actions the numbered users mean, by the bullet's row (each row has
-- one subtype: its gun and the bullet's height, `dword_80ECC54`,
-- `dword_80ECC5C`): the Spreaders' (subtype 0) and SpoutCross's (2) are
-- the chips'; the link navis' weapons' (0x5F: subtype 2, row 7; 0x74: 1,
-- 8; 0x80: 2, 4) are made here until those weapons are definitions.
local function link(gun: AttachmentLook, z: number, row: number): Action
    return spreader.action { gun = gun, z = z, bullet = bullet.by_number(row) }
end
local BY_ROW: { [number]: Action } = {
    [3] = spreaders.spreadr1,
    [0xD] = spreaders.spout_charge,
    [7] = link(spreader.guns.water_cannon, 0, 7),
    [8] = link(spreader.guns.spreader, 0x10_0000, 8),
    [4] = link(spreader.guns.water_cannon, 0, 4),
}

function by_number.update(me: Object, s: any)
    local row = me:attack_param(1)
    local a = BY_ROW[row]
    if not a then
        error(string.format("spreader with bullet row %#x: nothing sets it up", row))
    end
    a.update(me, s)
end

return by_number
