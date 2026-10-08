--!strict
-- A die (EXE5's attack object #5, 0x080C4F2C; EXE5's own: EXE6's `sub_80C55B0` family is another object):
-- @exelib/numbrman/dice, with EXE5's look and sounds; its face 9 (palette 1, the explosion of effect row 0x24,
-- animation 7); its markers over its target (lib/navi_chips/marker), in the operation battle only; its burst keeping
-- the damage's top bit apart (0x080C51F2). NumberMan's (./navi) runs while dimmed, an attack at navis; the others'
-- (the 0x080EE804, 0x080F1652 and 0x080FA66A throws) a body the other side can hit, standing still while dimmed.

local collision = require("../../rules/collision")
local marker = require("../../lib/navi_chips/marker")

return require("@exelib/numbrman/dice").make {
    id = "numbrman/dice",
    sprite = asset.sprite("dice"),
    -- The effect table's rows: 0 (the explosion), 2 (the puff off the field), 0x14 (the battle's end), 0x24 (9's).
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    puff = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 },
    gone = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    nine = { explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 } },
    blow_up = asset.sound("hit-bomb-1"),
    bounce = asset.sound("bug-bomb-land"),
    burst = asset.sound("target-move"),
    -- NumberMan's: an attack at navis (rows 4 and 5); the others': a body the other side hits (rows 0x13 and 0x14).
    attack = collision.attack,
    hits_navis = collision.hits_navis,
    body = collision.own_body,
    hit_by = collision.hit_by_other_side,
    marker = marker.spawn,
    keeps_top_bit = true,
}
