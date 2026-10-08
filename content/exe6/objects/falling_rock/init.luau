--!strict
-- A rock falling on a panel (the original's attack object #0x1D, `sub_80C7D78`: @exelib/rock/falling, EXE6's kind):
-- it appears 104 pixels over its panel and falls, faster and faster; its hit region opens once it is under 30
-- pixels. What it hits breaks it into two chips (./chip); landing it breaks on a solid panel, or drops into a hole
-- with a puff. Spawned by `sub_80C7F20`: the rules' GroundCross barrage (`sub_8012CB2`, the role
-- `kinds.falling_rock`), with the panel, element and damage word. (The barrage's Param1, 3 to 1, nothing reads.)

local collision = require("../rules/collision")
local effects = require("../lib/effects")
local falling = require("@exelib/rock/falling")
local sparks = require("../lib/sparks")
local chip = require("@self/chip")

local falling_rock = {}

-- Its look: from 104 pixels, falling 1/8 pixel a tick faster each tick; a thrown attack that breaks, against navis,
-- hit modifier 1, the plain spark; a puff into a hole (effect 0x14), its chips with their crash.
falling_rock.kind = falling.make_kind {
    id = "falling-rock",
    sprite = asset.sprite("falling-rock"),
    height = 0x68_0000,
    gravity = 0x2000,
    is = collision.thrown_break,
    hits = collision.hits_navis,
    hit_mod = 1,
    spark = sparks.plain,
    puff = effects.puff,
    chips = chip,
    shatter_sound = asset.sound("falling-rock"),
}

return falling_rock
