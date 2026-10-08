--!strict
-- NumbrBl's NumberMan (EXE5's actor object #0x45, 0x080C2018; EXE6's #0x45 draws the user): @exelib/numbrbl/numberman
-- with EXE5's look: his sprite in palette 0, his appearing sound (0x94), his throw (animation 0x10), and EXE5's balls
-- (@exelib/numbrbl/ball: attack object #0x91, 0x080D895C).

local common_ball = require("@exelib/numbrbl/ball")
local common = require("@exelib/numbrbl/numberman")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")

-- The balls (EXE5's attack object #0x91, 0x080D895C).
local ball = common_ball.make {
    id = "numbrbl/ball",
    sprite = asset.sprite("number-ball"),
    sit_ticks = 0x1D,
    -- (0x112: 0x080D8A40 loads it as 19 + 255.)
    set_sound = asset.sound("place"),
    hit_sound = asset.sound("damage"),
    explosion = effects.explosion,
    is = collision.attack,
    hits = collision.hits_navis,
    -- 0x080D8ABA: parts 2 to 21.
    hidden_parts = 0x3FFF_FC00,
}

return common.make {
    id = "numbrbl/numberman",
    sprite = asset.sprite("numberman"),
    -- NumbrBl's: navi 9's count, 0x0801D737: 1, by the row of 0x0800DAE8 of the chip's level, 2.
    palette = 0,
    appear = asset.sound("appear"),
    throw_anim = 0x10,
    ball = ball,
}
