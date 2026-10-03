--!strict
-- DarkCirc (BN5 chip 0x0bb): a dark chip: a gun sight circles the enemy's area until A or 360 ticks, then leaves
-- seven shots on the panels it passes, each with the HP bug 0x54 (argument 0x32): the shared CircGun (@common/circgun,
-- the dimming handler's subtype 42, BN6's code rows for it: its variant, period 2, 7 shots, look 1) with BN5's look:
-- its sight (10-22), sounds, spark and the shots' hit types (BN5's rows 0xA and 0x2B), and its gun and shots on
-- their panels' centers at height 0 (0x080D76F0, where BN6's go 8 pixels down and up). Its use costs custom damage
-- (rules/light-dark).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local collision = require("../../rules/collision")
local controller = require("@common/circgun/controller")
local effects = require("@bn6/lib/effects")
local gun = require("@common/circgun/gun")
local shot = require("@common/circgun/shot")
local sparks = require("../../lib/sparks")

local LOOK: CircGunLook = {
    sprite = asset.sprite("bn5:sight"),
    sounds = {
        step = asset.sound("bn5:erase-man"),
        mark = asset.sound("bn5:mark"),
        hit = asset.sound("bn5:col-army-2"),
    },
    hits = { [0] = collision.thrown, [1] = collision.collision_2b },
    -- (What the hit reaches and its flash BN6's.)
    target = collision.hits_navis,
    flash = effects.flash,
    spark = sparks.plain,
    on_floor = true,
}

local SHOT = shot.make_kind("bn5:darkcirc/shot", LOOK)
local GUN = gun.make_kind("bn5:darkcirc/gun", LOOK, SHOT)
local CONTROLLER = controller.make("bn5:darkcirc/controller", GUN)

return define.chip {
    id = "bn5:darkcirc",
    codes = { "R" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 300,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "dark", "variable_damage" },
    slot_in_limit = 1,
    dimming = CONTROLLER.hook(controller.variant { period = 2, shots = 7, look = 1 }),
}
