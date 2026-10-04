--!strict
-- BN5's VDoll doll (attack object #0x7A, 0x080D5618): the shared one (@exelib/vdoll/doll, BN6's code) in BN5's look,
-- and BN5's differences:
--
-- - from the start (0x080D568C) a body of no side that anything reaches, hit modifier 3 (BN6's own object reached
--   by the other side, hit modifier 1, until it sets down); its setting down (BN5's action 6, 0x080D57D0) does
--   nothing;
-- - a hit whose types have any of 0x0C800002 (0x080D56D4) breaks it as damage 0, so it leaves with a puff, no
--   curse;
-- - its leaving (0x080D57F0): no blinking out, no absorbing; carried off, gone; not hurt, not hit or the battle
--   over, a puff; flying, gone; else the curse, with a puff 12 pixels up if a chip removed it (the flag word's
--   0x10000), else effect 0x24 16 up and the trap-vanish sound (0x107; BN6's code there is unreachable);
-- - NameID 0xE2: actor record 0xE2 (0x08014C94: version 3, AI index 0x21), its look 0x0801D6B0's row 0x15 (the
--   doll, animation 1, a ground shadow).
--
-- Its actions are BN6's numbers (BN5's table, 0x080D5704, has six shared entries: its setting down is BN5's 6).

local collision = require("../../rules/collision")
local common = require("@exelib/vdoll/doll")
local curse = require("./curse")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local SPRITE = asset.sprite("bn5:voodoo-doll")

return common.make {
    id = "bn5:vdoll/doll",
    sprite = SPRITE,
    identity = define.identity {
        id = "bn5:vdoll",
        class = "field_object",
        version = 3,
        ai_index = 0x21,
        look = { sprite = SPRITE, anim = 1, palette = 0, shadow = true },
    },
    is = collision.neutral_body,
    hits = collision.everything,
    hit_mod = 3,
    -- 0x08016C3C(0): hit effect 0.
    spark = sparks.plain,
    breaking_hits = 0x0C80_0002,
    poison_sound = asset.sound("bn5:panel-poison"),
    puff = effects.puff,
    trajectory = trajectory,
    curse = curse,
    bn5_leaving = {
        -- Effect 0x24 (BN5's table, 0x080DFC94): sheet 14-00 in palette 1.
        flash = define.effect { sprite = asset.sprite("bn5:explosion"), anim = 0, palette = 1 },
        sound = asset.sound("bn5:trap-vanish"),
    },
}
