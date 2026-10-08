--!strict
-- NumbrBl1 to NumbrBl3: the dimming chips' variant 0x3A, EXE5's NumbrBl. A controller (effect object #0x69, 0x080E9838:
-- @exelib/numbrbl/controller, its effect EXE5's, 0x080E9754, with EXE4's warp: lib/navi_chips) on which the user warps
-- out, NumberMan himself (actor object #0x45, 0x080C8CC4: @exelib/numbrbl/numberman) sets the chip's balls (its first
-- parameter and three: 3, 4, 5) rolling down the row (attack object #0x91, 0x080DF4E4: @exelib/numbrbl/ball), and the
-- user warps back in. Their damage is the last two digits of the user's HP (power 1049: formula 49, 0x0801963E).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local common = require("@exelib/numbrbl/controller")
local common_ball = require("@exelib/numbrbl/ball")
local common_numberman = require("@exelib/numbrbl/numberman")
local navi_chips = require("../lib/navi_chips")

-- The balls (0x080DF508): sheet 10-05, sitting 30 ticks with 0xAA, an explosion (effect row 0) with 0xC8 as one
-- hits; parts 2 to 21 hidden as they wait and roll (0x080DF640).
local BALL = common_ball.make {
    id = "numbrbl/ball",
    sprite = asset.sprite("number-ball"),
    sit_ticks = 0x1D,
    set_sound = asset.sound("place"),
    hit_sound = asset.sound("damage"),
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    is = collision.attack,
    hits = collision.hits_navis,
    hidden_parts = 0x3FFF_FC00,
}

-- NumberMan (0x080C8CE8): sheet 08-08 in palette 0 (0x0800B950 by navi 8 and the chip's level 0), appearing with
-- 0xB0, throwing in his animation 7; his init loads his standing animation.
local NUMBERMAN = common_numberman.make {
    id = "numbrbl/numberman",
    sprite = asset.sprite("numberman"),
    palette = 0,
    appear = asset.sound("appear"),
    throw_anim = 7,
    ball = BALL,
    loads_standing = true,
}

local CONTROLLER = common.make(
    "numbrbl/controller",
    common.warping(function(me: Object, out: boolean)
        -- 0x080E2D26: the user's side's navi.
        navi_chips.warp(me.alliance, out)
    end, function(me: Object, damage: number, balls: number)
        -- 0x080C8E98.
        NUMBERMAN.spawn(me, me.panel_x, me.panel_y, damage, balls)
    end)
)

-- NumbrBl1 (EXE4 chip 0x0AC): three balls.
local numbrbl1: Chip = {
    damage = { formula = "hp_last_digits" },
    codes = { "E", "L", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 28,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library", "variable_damage" },
    dimming = common.hook(CONTROLLER, 3),
}

-- NumbrBl2 (EXE4 chip 0x0AD): four balls.
local numbrbl2: Chip = {
    damage = { formula = "hp_last_digits" },
    codes = { "A", "E", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 35,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library", "variable_damage" },
    dimming = common.hook(CONTROLLER, 4),
}

-- NumbrBl3 (EXE4 chip 0x0AE): five balls.
local numbrbl3: Chip = {
    damage = { formula = "hp_last_digits" },
    codes = { "L", "T", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 42,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library", "variable_damage" },
    dimming = common.hook(CONTROLLER, 5),
}

return { numbrbl1 = numbrbl1, numbrbl2 = numbrbl2, numbrbl3 = numbrbl3 }
