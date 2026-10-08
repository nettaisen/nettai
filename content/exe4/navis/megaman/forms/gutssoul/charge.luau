--!strict
-- GutsSoul's charged shot (`gutssoul/charged-shot`): weapon routine 0x24 (0x0800CF62: 60 damage, counter byte 0xA8,
-- every other attack variable 0) and the punch it starts (action 0x12, lib/punch): GutsMan's fist at the panel ahead,
-- its attack parameters 0x04050601 (region 1, the anchor alone; spark 6, the heavy hit's; an attack at navis).

local collision = require("../../../../rules/collision")
local punch = require("../../../../lib/punch")
local regions = require("@exelib/regions")
local sparks = require("../../../../lib/sparks")

local charge = {}

charge.action = punch.make {
    id = "gutssoul/charged-shot/action",
    region = regions.single,
    spark = sparks.heavy,
    target = collision.hits_navis,
    self_type = collision.attack,
}

-- 0x0800CF62.
local function setup(navi: Object): Action
    navi.attack_damage = 60
    navi.attack_element = 0
    navi.attack_variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0xA8
    return charge.action
end

charge.weapon = new.weapon {
    id = "gutssoul/charged-shot",
    -- The charge table's row 0x24 (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x25's
    -- first entry.
    charge_ticks = { 100, 100, 100, 100, 100, 60 },
    setup = setup,
}

return charge
