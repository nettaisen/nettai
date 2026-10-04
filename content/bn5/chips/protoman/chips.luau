--!strict
-- ProtoMan, ProtoMnSP and ProtoMnDS: the protoman series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("../../objects/attachment/attachment")
local common = require("@exelib/protoman/navi")
local navi = require("./navi")

-- ProtoMan (BN5 chip 0x0ef): ProtoMan cuts in and slashes each enemy navi he can reach: the shared navi
-- (@exelib/protoman/navi, BN5's kind ./navi; BN5's the same code but for his sprite, 08-01, BN6's 08-0b) with BN5's
-- look: his sword
-- BN5's attachment row 3 (BN6's 0x11, the same look) in its animation 1 (BN6 0xB, 0x080C199C's rows), his slash
-- BN5's effect 0x33 (BN6's 0x27, the same row), his palette 0 (0x080C1714) and his spawner BN5's 0x080C1A00.

local LOOK = common.look {
    sprite = asset.sprite("protoman"),
    palette = 0,
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
    sword_anim = 1,
    slash = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

local protoman = define.chip {
    id = "protoman",
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

-- ProtoMnSP (BN5 chip 0x0f0): ProtoMan cuts in and slashes each enemy navi he can reach, as ProtoMan's (the shared navi
-- with BN5's look: chips/protoman/chips.luau; his palette by the chip's Param1, 0x080C1714: 0 for its 3); its damage by how
-- fast its user deleted ProtoMan SP (rules/sp-chips: formula 2, its table's row 1).

local LOOK_PROTOMN_SP = common.look {
    sprite = asset.sprite("protoman"),
    palette = 0,
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
    sword_anim = 1,
    slash = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

local protomn_sp = define.chip {
    id = "protomn-sp",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 79,
    damage = { formula = "sp_navi", slot = "sp/protoman", by_time = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }, operation_battle = 220 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_with(LOOK_PROTOMN_SP),
}

-- ProtoMnDS (BN5 chip 0x0f1): ProtoMan, dark, cuts in and slashes each enemy navi he can reach, as ProtoMan's (the
-- shared navi with BN5's look: chips/protoman/chips.luau; his palette by the chip's Param1, 0x080C1714: 3 for its 4); its
-- damage by its user's side's statistic 3 (formula 24, its table's row 1).

local LOOK_PROTOMN_DS = common.look {
    sprite = asset.sprite("protoman"),
    palette = 3,
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
    sword_anim = 1,
    slash = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

local protomn_ds = define.chip {
    id = "protomn-ds",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 79,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250 }, operation_battle = 180 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_with(LOOK_PROTOMN_DS),
}

return { protoman = protoman, protomn_sp = protomn_sp, protomn_ds = protomn_ds }
