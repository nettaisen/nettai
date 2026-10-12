--!strict
-- ProtoSoul's B+Left special (`protosoul/reflect`): weapon routine 0x6A (0x0800D3C0), which its status reset puts in
-- AIData +0x00 (0x0800E12C): EXE4's guard (action 0x25, lib/guard) as the Reflect program's (variant 4: a hit it turns
-- aside is fired back as the charged shot's projectile), 50 damage, 20 ticks behind the shield's look row 1 (animation
-- 2; its parameters 0x0114); it sets the B+Left special's cooldown to 40 as it starts (AIAttackVars +0x1A).

local collision = require("../../../../rules/collision")
local guard = require("../../../../lib/guard")
local projectile = require("../../../../objects/projectile")
local shield = require("../../../../objects/shield")
local sparks = require("../../../../lib/sparks")

local reflect = {}

local TICKS = 0x14
local COOLDOWN = 40
-- What it fires back: the charged shot's projectile (row 6, 0x080ECEC2), 20 pixels up.
local REFLECTED = projectile.variant {
    id = "protosoul/reflect/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
}
local REFLECTED_Z = 0x14_0000

reflect.action = guard.action {
    id = "protosoul/reflect/action",
    ticks = TICKS,
    shield = shield.look { anim = 2 },
    counter = function(me: Object, x: number, y: number, damage: number)
        projectile.spawn(me, x, y, { variant = REFLECTED, damage = damage, z = REFLECTED_Z })
    end,
}

-- 0x0800D3C0.
reflect.weapon = new.weapon {
    id = "protosoul/reflect",
    -- The charge table's row 0x6A (0x080180B8): 60 ticks throughout (a B+Left special, which nothing charges).
    charge_ticks = { 60, 60, 60, 60, 60, 60 },
    setup = function(navi: Object): Action
        navi.attack_element = 0
        navi.attack_variant = 4
        navi.charged = 0
        navi.attack_lockout = 0
        navi.extra = 0
        navi.hit_param = 0
        navi.attack_damage = 50
        navi.back_special_cooldown = COOLDOWN
        return reflect.action
    end,
}

return reflect
