--!strict
-- EXE4's ice cube (attack object #0xA3, 0x080E13FC): the shared ice cube (@exelib/ice_cube, EXE6's code) in EXE4's
-- look: the rock's sheet (10-02: forming, animation 0; formed, 2), formed with the place sound (0xAA) and bursting with
-- the falling rock's (0xA3), the aqua spark, a neutral obstacle that everything reaches (rows 14 and 15), pushed by
-- its final hit modifier's bits 0xFC (0x080E14FC: EXE4's pushes take 0x40 and 0x80 too), bursting into the rock's
-- debris and dust (effect 2), gone off solid ground in a puff (effect 21). ColdMan forms one ahead of him and punches
-- it (chips/coldman); an obstacle he can reach becomes one, formed, with his side and damage (the role
-- `hooks.encased`, 0x08011788); one he can't, an ice block (EXE4's rock's row 3, 0x080D89C0: animation 2, 100 HP,
-- aqua, its debris in palette 1, the falling rock's sound).

local collision = require("../rules/collision")
local common = require("@exelib/ice_cube")
local rock = require("./rock")
local rock_lib = require("../lib/rock")
local sparks = require("../lib/sparks")

-- The ice block: row 3 of 0x080D89C0.
local ICE_BLOCK = rock.variant {
    anim = 2,
    hp = 100,
    element = "aqua",
    debris_palette = 1,
    break_sound = asset.sound("falling-rock"),
    identity = nil,
}

return common.make {
    id = "ice-cube",
    sprite = asset.sprite("rock"),
    sounds = { form = asset.sound("place"), burst = asset.sound("falling-rock") },
    spark = sparks.aqua,
    is = collision.neutral_body,
    hits = collision.everything,
    pushes = 0xFC,
    debris = rock_lib.debris,
    effects = { burst = rock_lib.effects.dust, puff = rock_lib.effects.puff },
    ice_block = { spawn = rock.spawn, variant = ICE_BLOCK },
}
