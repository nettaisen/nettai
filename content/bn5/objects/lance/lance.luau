--!strict
-- BN5's lance (attack object #0x4F, BN6's #0x6F): BN6's (@common/lance/lance, the same code) but for its sprite
-- (0C-2B), sound, and hit (0x080CF904): an attack against attacks of hit modifier 0x10 (BN6's 0x51) with the wood
-- spark; a lance of Param1 1 (DrkLance's, in its second palette) hits as BN5's row 0x25 with bug 0x16 (argument 2).

local collision = require("../../rules/collision")
local lance = require("@common/lance/lance")
local sparks = require("../../lib/sparks")

return {
    kind = lance.make_kind("bn5:lance", {
        sprite = asset.sprite("bn5:lance"),
        sound = asset.sound("bn5:sword-swing"),
        hits = {
            [0] = { is = collision.attack, hits = collision.attack, spark = sparks.wood, hit_mod = 0x10 },
            [1] = {
                is = collision.attack_480,
                hits = collision.attack,
                spark = sparks.wood,
                hit_mod = 0x10,
                bug = 0x16,
                bug_arg = 2,
            },
        },
    }),
}
