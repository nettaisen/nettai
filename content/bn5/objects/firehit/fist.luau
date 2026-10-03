--!strict
-- BN5's FireHit fist (attack object 0x5B, 0x080D131C) and the chips' effect (action 0x1A's effect 9, 0x080D1436):
-- BN6's code (@bn6/chips/firehit/fist), the fist striking with BN5's fire spark and collision types (the strike's
-- word 0x04050101, BN5's rows 5 and 4).
--
-- Not ported: in battle flag 0x40 mode (never in a netbattle), BN5's warning also spawns effect 0x83 on the fist's
-- panel for 10 ticks (0x080E9FA4, from 0x080D13B2; BN6 stubs the call), whose object is unread.

local bn6_fist = require("@bn6/chips/firehit/fist")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local fist = {}

fist.kind = bn6_fist.make_kind("bn5:firehit/fist", {
    spark = sparks.fire,
    target = collision.hits_navis,
    self_type = collision.attack,
})

-- The chips' `instant` hook: the fists hit with modifier 3 (the record's +0x11).
fist.instant = bn6_fist.instant(3, fist.kind)

return fist
