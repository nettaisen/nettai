--!strict
-- The buster patches' B buttons (weapon routines 0x34, 0x35, 0x5A, 0x68 and 0x69, 0x0800D0B8 on: Attack + 1 damage,
-- every other attack variable 0, the attack parameter the projectile row; the patch cards 43, 44, 12, 13 and 102): the
-- buster's shot (action 8, ./buster's `shot_action`) firing projectile rows 7, 8, 0x0B, 0x0D and 0x0E (0x080CD2A8):
-- the plain shot whose hit cracks its panel (row 7: 0x080CD430), or turns a solid one to poison, grass or ice (its
-- ninth byte, 0x080CD440), or that blinds for 1200 ticks (row 0x0B: status 0x32, 0x080CD3BC).

local buster = require("./buster")
local collision = require("../../../rules/collision")
local shot = require("../../../objects/projectile")
local sparks = require("../../../lib/sparks")
local statuses = require("../../../rules/status")

local buster_patches = {}

-- The charge table's rows 0x34, 0x35, 0x5A, 0x68 and 0x69: 60 ticks at every Charge (a B button's, never charged).
local SIXTY: { number } = { 60, 60, 60, 60, 60, 60 }

-- A plain shot's row (0x080CD2A8: an attack hitting navis, hit modifier 0, null, the plain spark, not drawn) with what
-- its hit does to the panel or its status.
local function row(id: string, hit_panel: PanelHit?, status: Status?): ProjectileVariant
    return shot.variant {
        id = id,
        collision = { is = collision.attack, hits = collision.hits_navis },
        hit_mod = 0,
        element = "null",
        spark = sparks.plain,
        hit_panel = hit_panel,
        status = status,
    }
end

-- 0x0800D0B8 and its kind: Attack + 1 damage, the rest 0 (the row its action fires).
local function setup(action: Action): (navi: Object) -> Action
    return function(navi: Object): Action
        navi.attack_damage = battle.navi(navi.alliance).attack + 1
        navi.attack_element = 0
        navi.charged = 0
        navi.extra = 0
        navi.hit_param = 0
        navi.attack_lockout = 0
        return action
    end
end

-- The weapon `id` firing `variant` from the buster.
local function patch(id: string, variant: ProjectileVariant): Weapon
    local action = buster.shot_action(id .. "/shot", variant)
    return new.weapon { id = id, charge_ticks = SIXTY, setup = setup(action) }
end

-- Routine 0x34 (card 43): row 7, cracking.
buster_patches.crack = patch("megaman/crack-buster", row("megaman/crack-buster/row", { effect = "crack" }, nil))
-- Routine 0x35 (card 44): row 8, poison.
buster_patches.poison = patch(
    "megaman/poison-buster",
    row("megaman/poison-buster/row", { effect = "set_type", left_side = "poison", right_side = "poison" }, nil)
)
-- Routine 0x5A (card 12): row 0x0B, blinding.
buster_patches.blind = patch("megaman/blind-buster", row("megaman/blind-buster/row", nil, statuses.blind_1200))
-- Routine 0x68 (card 13): row 0x0D, grass.
buster_patches.grass = patch(
    "megaman/grass-buster",
    row("megaman/grass-buster/row", { effect = "set_type", left_side = "grass", right_side = "grass" }, nil)
)
-- Routine 0x69 (card 102): row 0x0E, ice.
buster_patches.ice = patch(
    "megaman/ice-buster",
    row("megaman/ice-buster/row", { effect = "set_type", left_side = "ice", right_side = "ice" }, nil)
)

return buster_patches
