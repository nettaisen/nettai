--!strict
-- KnightSoul (soul 11, Team Colonel's): BN5's Soul Unison with KnightMan, for a Break chip given up. MegaMan's sprite
-- in it is 00-0B (0x0800DA58[11]), his element Null (0x0800E634 by soul); he wears nothing over it (KnightMan's image
-- wears his ball and chain). Its weapons (0x0801CA1C's row 11, loaded by 0x0800DCA0): the buster (routine 0), the
-- wrecking ball (0x11, ./charge), Break chips charged with A (0x12, empty, 0x0800F7D6: any Break chip but a dimming
-- or dark one, 0x0801090A; their damage doubled, 0x080103D0), Chaos Unison's (0x38: DarkDril); no B+Back (0xFF). A
-- damaging chip (not a dimming one) used from his area's front edge (the panel ahead not his side's) keeps him
-- invulnerable for 50 ticks (0x08010392).

local attach_points = require("../attach_points")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_chips = require("../weapons/charged-chip")
local darkdril = require("../../../chips/darkdril")
local souls = require("../../../rules/souls")

return define.form {
    id = "knightsoul",
    soul = { number = 11, family = "break" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("knightsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- Routine 0x12 (the charged chip is the chip with the bonus), the charge table's row 0x12.
        a_charge = charged_chips.routine("knightsoul/a-charge", { 80, 80, 80, 80, 80, 120 }),
        -- Routine 0x38 (0x0800FA84): DarkDril (chip 0xC4), the charge table's row 0x38.
        chaos = chaos.weapon("knightsoul/chaos", darkdril, { 40, 60, 90, 120, 150, 40 }),
    },
    charged_chips = { { family = "break", damaging = false, plain = true } },
    charged_bonus = { damage = 0xFF },
    front_guard = 50,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 0x16-0x1B, Team Colonel's
    -- souls'); in its Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("knightsoul"),
        variant = { normal = asset.mugshot("knightsoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 11 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[11],
    },
}
