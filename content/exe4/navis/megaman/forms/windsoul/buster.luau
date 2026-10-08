--!strict
-- WindSoul's B button (`windsoul/buster`): weapon routine 0x6B (0x0800D3E8): AirShot's attack (chip 0x004, loaded as
-- 0x0800D406 loads a chip: its parameters, element, variant, lockout) with 5 damage, counter byte 0x85 and variant 1,
-- and AirShot's action (0x23, lib/airshot), whose recovery that variant makes 30 ticks (0x080ECC02: a variant not 0).
-- Its shot is AirShot's: the projectile row 4 that the chip's parameter +0x10 names.

local airshot = require("../../../../lib/airshot")
local airshot_chip = require("../../../../chips/airshot").airshot
local collision = require("../../../../rules/collision")
local projectile = require("../../../../objects/projectile")
local sparks = require("../../../../lib/sparks")

local buster = {}

-- Projectile row 4 (action 0x23's, 0x080ECBEE): it pushes (hit modifier 0x21), with the impact spark.
local SHOT = projectile.variant {
    id = "windsoul/buster/shot",
    collision = { is = collision.pushing, hits = collision.hits_navis },
    hit_mod = 0x21,
    element = "null",
    spark = sparks.impact,
}

buster.action = airshot.action { id = "windsoul/buster/action", shooter = airshot.shooter, shot = SHOT, recovery = 30 }

-- 0x0800D3E8.
local function setup(navi: Object): Action
    navi:load_chip_attack(airshot_chip)
    navi.attack_damage = 5
    navi.hit_param = 0x85
    navi.attack_variant = 1
    return buster.action
end

buster.weapon = new.weapon {
    id = "windsoul/buster",
    -- The charge table's row 0x6B (0x080180B8), by Charge 0 to 4; Charge 5 reads on into row 0x6C's first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 255 },
    setup = setup,
}

return buster
