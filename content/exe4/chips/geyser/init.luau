--!strict
-- Geyser (EXE4 chip 0x02D; its record at 0x080197EC + 0x2D * 0x2C): the throw, action 0x09's variant 5: the navi holds
-- the bomb (attachment row 4, its animation 0: 0x080EB4EC) and lobs an aqua bomb (attack object 0x42, 0x080D58EE,
-- thrown by 0x080D5B62) at the panel three ahead: on solid ground a splash of 10 damage and the throw's Atk+ bonus; in
-- a hole, a geyser that hits its panel and sends water (attack object 0x43, 0x080D5B84) onto the eight around it: the
-- shared one (@exelib/geyser/geyser, EXE5's Geyser's code but for its constants and the bonus) with EXE4's look: the
-- bomb's sprite and the geyser's (0c-25), effect row 0x25 (a splash) and its sound (0x9F), the eruption's sound
-- (0x120), aqua hits (an attack reaching navis, the aqua spark), its water's height the battle state's address (so
-- the water's hits' sparks show far above the field).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bombs = require("../lib/bombs")
local collision = require("../rules/collision")
local common = require("@exelib/geyser/geyser")
local sparks = require("../lib/sparks")
local trajectory = require("../lib/trajectory")

local GEYSER = common.make({
    id = "geyser/bomb",
    water_id = "geyser/water",
    bomb = asset.sprite("bomb"),
    geyser = asset.sprite("geyser"),
    splash = new.effect { sprite = asset.sprite("bubble-burst"), anim = 0, palette = 0 },
    splash_sound = asset.sound("bubble-burst"),
    erupt = asset.sound("geyser"),
    attack = collision.attack,
    hits = collision.hits_navis,
    spark = sparks.aqua,
    splash_bonus = true,
    -- The battle state's address (0x08006100: the battle-over test's r3, which the water's spawn keeps as its height).
    water_height = 0x0203_5810,
}, trajectory)

local geyser: Chip = {
    action = bombs.throw { id = "geyser/action", held = bombs.held, thrower = GEYSER.thrower() },
    codes = { "B", "L", "V" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 38,
    damage = 200,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { geyser = geyser }
