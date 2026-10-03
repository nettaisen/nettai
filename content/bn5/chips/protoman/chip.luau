--!strict
-- ProtoMan (BN5 chip 0x0ef): ProtoMan cuts in and slashes each enemy navi he can reach: the shared navi
-- (@common/protoman/navi, BN5's kind ./navi; BN5's the same code but for his sprite, 08-01, BN6's 08-0b) with BN5's
-- look: his sword
-- BN5's attachment row 3 (BN6's 0x11, the same look) in its animation 1 (BN6 0xB, 0x080C199C's rows), his slash
-- BN5's effect 0x33 (BN6's 0x27, the same row), his palette 0 (0x080C1714) and his spawner BN5's 0x080C1A00.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("../../objects/attachment/attachment")
local common = require("@common/protoman/navi")
local navi = require("./navi")

local LOOK = common.look {
    sprite = asset.sprite("bn5:protoman"),
    palette = 0,
    sword = attachment.look { sprite = asset.sprite("bn5:sword"), palette = 0 },
    sword_anim = 1,
    slash = define.effect { sprite = asset.sprite("bn5:sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

return define.chip {
    id = "bn5:protoman",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 2,
    mb = 54,
    damage = 130,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_with(LOOK),
}
