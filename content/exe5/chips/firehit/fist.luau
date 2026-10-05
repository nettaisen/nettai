--!strict
-- EXE5's FireHit fist (attack object 0x5B, 0x080D131C) and the chips' effect (action 0x1A's effect 9, 0x080D1436):
-- the shared code (@exelib/firehit/fist, EXE6's), the fist striking with EXE5's fire spark and collision types (the
-- strike's word 0x04050101, EXE5's rows 5 and 4); its sprite and sound EXE6's numbers.
--
-- Not ported: in the operation battle (never in a netbattle), EXE5's warning also spawns effect 0x83 on the fist's
-- panel for 10 ticks (0x080E9FA4, from 0x080D13B2; EXE6 stubs the call), whose object is unread.

local common = require("@exelib/firehit/fist")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local fist = {}

fist.kind = common.make_kind("firehit/fist", {
    sprite = asset.sprite("fire-hit"),
    sound = asset.sound("fire-hit"),
    spark = sparks.fire,
    target = collision.hits_navis,
    self_type = collision.attack,
})

-- The chips' `instant` hook: the fists hit with modifier 3 (the record's +0x11).
fist.instant = common.instant(3, fist.kind)

return fist
