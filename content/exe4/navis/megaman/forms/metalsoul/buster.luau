--!strict
-- MetalSoul's B button (`metalsoul/buster`): weapon routine 0x0B (0x0800CDE2: the buster's, 0x0800CC2E, Attack + 1
-- damage and every other attack variable 0, but the attack parameter 5) and the buster's shot (action 8, its
-- `shot_action`) firing projectile row 5 (0x080CD2A8): a breaking shot.

local buster = require("../../weapons/buster")
local collision = require("../../../../rules/collision")
local shot = require("../../../../objects/projectile")
local sparks = require("../../../../lib/sparks")

local metal_buster = {}

-- Projectile row 5: a breaking attack (collision row 6) hitting navis, hit modifier 0, null, the breaking spark (0x0A),
-- not drawn.
local SHOT = shot.variant {
    id = "metalsoul/buster/row",
    collision = { is = collision.breaking, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.breaking,
}

metal_buster.action = buster.shot_action("metalsoul/buster/shot", SHOT)

-- 0x0800CDE2.
local function setup(navi: Object): Action
    navi.attack_damage = battle.navi(navi.alliance).attack + 1
    navi.attack_element = 0
    navi.attack_variant = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return metal_buster.action
end

metal_buster.weapon = new.weapon {
    id = "metalsoul/buster",
    -- The charge table's row 0x0B (0x080180B8): 40 ticks at every Charge (a B button's, never charged).
    charge_ticks = { 40, 40, 40, 40, 40, 40 },
    setup = setup,
}

return metal_buster
