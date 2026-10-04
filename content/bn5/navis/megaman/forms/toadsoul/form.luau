--!strict
-- ToadSoul (soul 12, Team Colonel's): BN5's Soul Unison with ToadMan, for an Aqua chip given up. MegaMan's sprite in
-- it is 00-0C (0x0800DA58[12]), his element Aqua (0x0800E634 by soul); he wears nothing over it. Its weapons
-- (0x0801CA1C's row 12, loaded by 0x0800DCA0): the buster (routine 0), the horn's notes (0x2D, ./charge), Aqua chips
-- charged with A (0x1B: any Aqua chip but a dimming or dark one, 0x0801090A; their damage doubled, 0x080103D0),
-- Chaos Unison's (0x34: DarkWide); no B+Back (0xFF). Its status reset has him dive (0x08011CB2: AIData +0x44's 0x20,
-- which every status reset clears first: the sea submerges him and doesn't hold him at a move's end).

local attach_points = require("../../attach_points")
local buster = require("../../weapons/buster")
local chaos = require("../../weapons/chaos")
local charge = require("./charge")
local charged_chips = require("../../weapons/charged-chip")
local darkwide = require("../../../../chips/darkwide/chip")
local souls = require("../../../../rules/souls/system")

return define.form {
    id = "toadsoul",
    soul = { number = 12, family = "aqua" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("toadsoul"),
    element = "aqua",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- Routine 0x1B (0x0800F520, empty: the charged chip is the chip with the bonus), the charge table's row 0x1B.
        a_charge = charged_chips.routine("toadsoul/a-charge", { 20, 20, 20, 20, 20, 20 }),
        -- Routine 0x34 (0x0800FA5C): DarkWide (chip 0xC0), the charge table's row 0x34.
        chaos = chaos.weapon("toadsoul/chaos", darkwide, { 40, 60, 90, 120, 150, 40 }),
    },
    charged_chips = { { family = "aqua", damaging = false, plain = true } },
    charged_bonus = { damage = 0xFF },
    reset = function(navi: Object)
        navi:set_navi_state("dives", true)
    end,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 0x16-0x1B, Team Colonel's
    -- souls'); in its Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("toadsoul"),
        variant = { normal = asset.mugshot("toadsoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 12 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[12],
    },
}
