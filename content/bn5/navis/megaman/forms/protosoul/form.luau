--!strict
-- ProtoSoul (soul 1, Team ProtoMan's): BN5's Soul Unison with ProtoMan, for a sword chip given up. MegaMan's sprite
-- in it is 00-01 (0x0800DA58[1]); he changes by BN5's soul change (rules/souls). Its weapons (0x0801CA1C's row 1,
-- loaded by 0x0800DCA0): the buster (routine 0), the charged slash (3, ./charge), Sword chips charged with A (5,
-- the A-charge's chips 0x0801090A: any Sword chip but a dimming or dark one; their damage doubled, 0x080103D0).
-- Not yet: its B+Back shield (routine 4, BN5's guard action 0x1F) and Chaos Unison's charge (AIData +0x11's routine
-- 6, DrkSword).

local buster = require("../../weapons/buster")
local charge = require("./charge")
local charged_chip = require("../../weapons/charged-chip")
local souls = require("../../../../rules/souls/system")

return define.form {
    id = "bn5:protosoul",
    kind = "soul",
    soul = { number = 1, family = "sword" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("bn5:protosoul"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = charged_chip.weapon },
    charged_chips = { { family = "sword", damaging = false, plain = true } },
    charged_bonus = { damage = 0xFF },
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls').
    mugshot = asset.mugshot("bn5:protosoul"),
    buster_arm = { anim = 0 },
}
