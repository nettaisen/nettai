--!strict
-- Ball (EXE4 chip 0x02C; its record at 0x080197EC + 0x2C * 0x2C): the throw, action 0x09's variant 4: the navi holds
-- the bomb in palette 4 (attachment row 0x2A, its animation 0: 0x080EB4EC) and lobs a cannonball (attack object 0x35,
-- 0x080D384C, thrown by 0x080D3A58) at the panel three ahead, which breaks the panel where it lands: the shared one
-- (@exelib/cannball/ball, EXE5's CannBall's code but for its constants) with EXE4's look: the bomb's sprite, effect
-- rows 0x15 (a puff) and 0x12 (an impact), the crack's sound (0x95), a thrown object that breaks, hitting navis.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local bombs = require("../lib/bombs")
local collision = require("../rules/collision")
local common = require("@exelib/cannball/ball")
local trajectory = require("../lib/trajectory")

local SPRITE = asset.sprite("bomb")

local BALL = common.make({
    id = "ball/ball",
    sprite = SPRITE,
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    impact = new.effect { sprite = asset.sprite("hit-sparks"), anim = 1, palette = 0 },
    crack = asset.sound("panel-crack"),
    is = collision.thrown_break,
    hits = collision.hits_navis,
}, trajectory)

local ball: Chip = {
    action = bombs.throw {
        id = "ball/action",
        held = attachment.look { sprite = SPRITE, palette = 4 },
        thrower = BALL.thrower(),
    },
    codes = { "B", "T", "V", "*" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 140,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { ball = ball }
