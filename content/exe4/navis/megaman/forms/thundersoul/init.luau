--!strict
-- ThunderSoul (soul 6): EXE4's Double Soul with ThunderMan, for an elec chip given up (0x08020008[5]).
-- MegaMan's sprite in it is 00-06, his face picture 10 (0x08014B78), ThunderMan's image thunderman (08-06).
-- Its element is elec (0x0800C964). Its weapons (0x080184F0's row 6): the buster on B, no A charge, the zap ring
-- charged (routine 0x58, ./charge). Its damaging Null and Elec chips, not dimming chips, paralyze used uncharged
-- (0x0800C4C4). (What the souls share: forms/soul.)

local charge = require("@self/charge")
local chip = require("@self/chip").thunsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "elec", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("thunderman")),
    sprite = asset.sprite("thundersoul"),
    element = "elec",
    buster_bonus = 0,
    weapons = { buster = soul.buster, charge_shot = charge.weapon },
    paralyzing_chips = { "null", "elec" },
    palette = 0,
    mugshot = { normal = asset.mugshot("thundersoul") },
}

return { thundersoul = form }
