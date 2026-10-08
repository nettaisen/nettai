--!strict
-- ProtoMan, ProtoMnSP and ProtoMnDS: the protoman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local attachment = require("../objects/attachment")
local common = require("@exelib/protoman/navi")
local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")
local sp_chips = require("../rules/sp_chips")

-- ProtoMan (EXE5 chip 0x0ef): ProtoMan cuts in and slashes each enemy navi he can reach: the shared navi
-- (@exelib/protoman/navi, EXE5's kind ./navi; EXE5's the same code but for his sprite, 08-01, EXE6's 08-0b) with EXE5's
-- look: his sword
-- EXE5's attachment row 3 (EXE6's 0x11, the same look) in its animation 1 (EXE6 0xB, 0x080C199C's rows), his slash
-- EXE5's effect 0x33 (EXE6's 0x27, the same row), his palette 0 (0x080C1714) and his spawner EXE5's 0x080C1A00.

local LOOK = common.look {
    sprite = asset.sprite("protoman"),
    palette = 0,
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
    sword_anim = 1,
    slash = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

local protoman: Chip = {
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
    navi = navi_chips.make {
        id = "protoman/controller",
        summon = navi.summon_with(LOOK),
    },
    auto_battle = { class = "at_once" },
}

-- ProtoMnSP (EXE5 chip 0x0f0): ProtoMan cuts in and slashes each enemy navi he can reach, as ProtoMan's (the shared navi
-- with EXE5's look: chips/protoman/init.luau; his palette by the chip's Param1, 0x080C1714: 0 for its 3); its damage by how
-- fast its user deleted ProtoMan SP (rules/sp_chips: formula 2, its table's row 1).

local LOOK_PROTOMN_SP = common.look {
    sprite = asset.sprite("protoman"),
    palette = 0,
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
    sword_anim = 1,
    slash = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

local protomn_sp: Chip = {
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 79,
    sp = { by_time = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }, operation_battle = 220 },
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "protomn-sp/controller",
        summon = navi.summon_with(LOOK_PROTOMN_SP),
    },
    auto_battle = { class = "at_once" },
}

-- ProtoMnDS (EXE5 chip 0x0f1): ProtoMan, dark, cuts in and slashes each enemy navi he can reach, as ProtoMan's (the
-- shared navi with EXE5's look: chips/protoman/init.luau; his palette by the chip's Param1, 0x080C1714: 3 for its 4); its
-- damage by its user's side's statistic 3 (formula 24, its table's row 1). His sword is the dark one: the sword's word
-- by the same Param1 (0x080C199C: 0x00010103 for 0 to 3, 0x06010103 for 4) carries a palette in its top byte, which
-- the attachment adds to its row's (0x080B9B0A): 6, the sheet's dark blade (the attachment's own row 0x32's).

local LOOK_PROTOMN_DS = common.look {
    sprite = asset.sprite("protoman"),
    palette = 3,
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 6 },
    sword_anim = 1,
    slash = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

local protomn_ds: Chip = {
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
    navi = navi_chips.make {
        id = "protomn-ds/controller",
        summon = navi.summon_with(LOOK_PROTOMN_DS),
    },
    auto_battle = { class = "at_once" },
}

return { protoman = protoman, ["protomn-sp"] = protomn_sp, ["protomn-ds"] = protomn_ds }
