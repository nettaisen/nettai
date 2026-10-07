--!strict
-- DrkVulcn (EXE4 chip 0x0BE): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- vulcans' action 0x1F, its variant 4: the dark gun (attachment row 49) fires 24 shots, bullet row 14 (its parameter
-- +0x10): the panel found and the one behind it, modifier 1, the dark hit shown once. Its cost: 720 ticks of
-- confusion (rules/dark_chips).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bullet = require("../objects/bullet")
local regions = require("@exelib/regions")
local vulcan = require("./vulcan/vulcan")

-- Bullet row 14: the panel found and the one behind it, modifier 1, its dark hit (effect row 0x6A: the Vulcans' hit in
-- palette 2) shown once at the bullet.
local SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 1,
    effect = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 2 },
    single = true,
}

local drkvulcn: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 99,
    damage = 20,
    hit_param = 2,
    lockout = 0,
    flags = { "has_damage", "dark" },
    action = vulcan.action {
        id = "drkvulcn/action",
        gun = vulcan.guns.dark,
        shots = 24,
        shot = SHOT,
    },
}

return { drkvulcn = drkvulcn }
