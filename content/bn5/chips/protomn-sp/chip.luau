--!strict
-- ProtoMnSP (BN5 chip 0x0f0): ProtoMan cuts in and slashes each enemy navi he can reach, as ProtoMan's (BN6's navi
-- with BN5's look: ../protoman/chip.luau; his palette by the chip's Param1, 0x080C1714: 0 for its 3); its damage by
-- how fast its user deleted ProtoMan SP (rules/sp-chips: formula 2, its table's row 1).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("@bn6/objects/attachment/attachment")
local navi = require("@bn6/chips/protoman/navi")

local LOOK = navi.look {
    sprite = asset.sprite("bn5:protoman"),
    palette = 0,
    sword = attachment.look { sprite = asset.sprite("bn5:sword"), palette = 0 },
    sword_anim = 1,
    slash = define.effect { sprite = asset.sprite("bn5:sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

return define.chip {
    id = "bn5:protomn-sp",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 79,
    damage = { formula = "sp_navi", slot = "sp/protoman", by_time = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }, per_player_gauges = 220 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_with(LOOK),
}
