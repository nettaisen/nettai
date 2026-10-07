--!strict
-- EXE5's cannonball (attack object #0x35, 0x080CB5C8; thrown by action 0x12's subtype 4, 0x080CB7D4): the shared one
-- (@exelib/cannball/ball, EXE5's code, which EXE4's Ball shares) with EXE5's look: the bomb's sprite, effects 0x14 (a
-- puff) and 0x11 (an impact), the crack's sound, a thrown object that breaks, hitting navis.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/cannball/ball")
local trajectory = require("../../lib/trajectory")

local cannball = {}

-- 0x080CB5EC: the bomb's sprite.
local SPRITE = asset.sprite("bomb")

-- What the navi holds throwing one (the attachment table's row 0x1A): the bomb in palette 4.
cannball.held = attachment.look { sprite = SPRITE, palette = 4 }

local made = common.make({
    id = "cannball/ball",
    sprite = SPRITE,
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    impact = new.effect { sprite = asset.sprite("hit-sparks"), anim = 1, palette = 0 },
    crack = asset.sound("panel-crack"),
    is = collision.thrown_break,
    hits = collision.hits_navis,
}, trajectory)

-- A thrower of cannonballs at the panel three ahead of the user.
cannball.thrower = made.thrower

return cannball
