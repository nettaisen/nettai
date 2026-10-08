--!strict
-- Django, DjangoSP and DjangoDS: the django series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")

-- Django (EXE5 chip 0x116): Django cuts in, shuts the first enemy navi ahead in his row in his coffin and purifies it
-- under his sun (./navi): a hit a tick for the chip's damage in ticks.
local django: Chip = {
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 40,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    megaman = "light",
    -- (In the navi chips' block though the US record lacks the `navi` flag: AntiNavi turns it back, and it is kept
    -- as the last navi chip.)
    navi = navi_chips.make {
        id = "django/controller",
        summon = navi.summon,
        navi_block = true,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- DjangoSP (EXE5 chip 0x117): Django cuts in, shuts the first enemy navi ahead in his row in his coffin and purifies
-- it under his sun (../django/navi); its damage (the purification's ticks) by the turn (formula 22, 0x0800E98A: its
-- table's row 21, a step a turn from the first).
local django_sp: Chip = {
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 68,
    damage = { formula = "count", of = "turns_before", by_count = { 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250 }, operation_battle = 200 },
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "light",
    -- (In the navi chips' block though the US record lacks the `navi` flag: AntiNavi turns it back, and it is kept
    -- as the last navi chip.)
    navi = navi_chips.make {
        id = "django-sp/controller",
        summon = navi.summon_sp,
        navi_block = true,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- DjangoDS (EXE5 chip 0x118): Django, dark, cuts in, shuts the first enemy navi ahead in his row in his coffin and
-- purifies it under his sun (../django/navi); its damage (the purification's ticks) by its user's side's statistic 3
-- (formula 44, its table's row 21).
local django_ds: Chip = {
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 68,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250 }, operation_battle = 150 },
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    -- (In the navi chips' block though the US record lacks the `navi` flag: AntiNavi turns it back, and it is kept
    -- as the last navi chip.)
    navi = navi_chips.make {
        id = "django-ds/controller",
        summon = navi.summon_dark,
        navi_block = true,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { django = django, ["django-sp"] = django_sp, ["django-ds"] = django_ds }
