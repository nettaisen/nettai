--!strict
-- NapalmSoul (soul 4, Team ProtoMan's): BN5's Soul Unison with NapalmMan, for a Fire chip given up. MegaMan's sprite
-- in it is 00-04 (0x0800DA58[4]), his element Fire (0x0800E634 by soul); he wears nothing over it (NapalmMan's image
-- wears his cannon). Its weapons (0x0801CA1C's row 4, loaded by 0x0800DCA0): the buster (routine 0), the fire
-- bullets (0x19, ./charge), Fire chips charged with A (0x27: any Fire chip but a dimming or dark one, 0x0801090A;
-- their damage doubled, 0x080103D0, and each lobbed as a napalm bomb, ./charged-chip: 0x08010442), Chaos Unison's
-- (0x37: DarkMetr); no B+Back (0xFF).

local attach_points = require("../../attach_points")
local buster = require("../../weapons/buster")
local chaos = require("../../weapons/chaos")
local charge = require("./charge")
local charged_chip = require("./charged-chip")
local charged_chips = require("../../weapons/charged-chip")
local darkmetr = require("../../../../chips/darkmetr/chip")
local souls = require("../../../../rules/souls/system")

return define.form {
    id = "bn5:napalmsoul",
    soul = { number = 4, family = "fire" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("bn5:napalmsoul"),
    element = "fire",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- Routine 0x27 (0x0800F520, empty: the charged chip is the chip with the bonus), the charge table's row 0x27.
        a_charge = charged_chips.routine("bn5:napalmsoul/a-charge", { 120, 120, 120, 120, 120, 80 }),
        -- Routine 0x37 (0x0800FA7A): DarkMetr (chip 0xC3), the charge table's row 0x37.
        chaos = chaos.weapon("bn5:napalmsoul/chaos", darkmetr, { 70, 90, 110, 130, 150, 40 }),
    },
    charged_chips = { { family = "fire", damaging = false, plain = true } },
    charged_bonus = { damage = 0xFF },
    charged_action = charged_chip.action,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls'); in its
    -- Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("bn5:napalmsoul"),
        variant = { normal = asset.mugshot("bn5:napalmsoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 4 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[4],
    },
}
