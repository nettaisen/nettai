--!strict
-- MagnetSoul (soul 5, Team ProtoMan's): BN5's Soul Unison with MagnetMan, for an Elec chip given up. MegaMan's sprite
-- in it is 00-05 (0x0800DA58[5]), his element Elec (0x0800E634 by soul); he wears nothing over it. Its weapons
-- (0x0801CA1C's row 5, loaded by 0x0800DCA0): the buster (routine 0), the magnet's pull (0x15, ./charge), Elec chips
-- charged with A (0x16: any Elec chip but a dimming or dark one, 0x0801090A; their damage doubled, 0x080103D0),
-- its B+Back (0x25, ./back: a glow and immobilizers on the enemies ahead), Chaos Unison's (0x35: DarkThnd).

local attach_points = require("../../attach_points")
local back = require("./back")
local buster = require("../../weapons/buster")
local chaos = require("../../weapons/chaos")
local charge = require("./charge")
local charged_chips = require("../../weapons/charged-chip")
local darkthnd = require("../../../../chips/darkthnd/chip")
local souls = require("../../../../rules/souls/system")

return define.form {
    id = "bn5:magnetsoul",
    kind = "soul",
    soul = { number = 5, family = "elec" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("bn5:magnetsoul"),
    element = "elec",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- Routine 0x16 (0x0800F520, empty: the charged chip is the chip with the bonus), the charge table's row 0x16.
        a_charge = charged_chips.routine("bn5:magnetsoul/a-charge", { 80, 80, 80, 80, 80, 70 }),
        back_special = back.weapon,
        -- Routine 0x35 (0x0800FA66): DarkThnd (chip 0xC1), the charge table's row 0x35.
        chaos = chaos.weapon("bn5:magnetsoul/chaos", darkthnd, { 40, 60, 90, 120, 150, 70 }),
    },
    charged_chips = { { family = "elec", damaging = false, plain = true } },
    charged_bonus = { damage = 0xFF },
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls'); in its
    -- Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("bn5:magnetsoul"),
        variant = { normal = asset.mugshot("bn5:magnetsoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 5 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[5],
    },
}
