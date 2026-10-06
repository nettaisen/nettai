--!strict
-- ToadSoul (soul 12, Team Colonel's): EXE5's Soul Unison with ToadMan, for an Aqua chip given up. MegaMan's sprite in
-- it is 00-0C (0x0800DA58[12]), his element Aqua (0x0800E634 by soul); he wears nothing over it. Its weapons
-- (0x0801CA1C's row 12, loaded by 0x0800DCA0): the buster (routine 0), the horn's notes (0x2D, ./charge), Aqua chips
-- charged with A (0x1B: any Aqua chip but a dimming or dark one, 0x0801090A; their damage doubled, 0x080103D0),
-- Chaos Unison's (0x34: DarkWide); no B+Back (0xFF). Its status reset has him dive (0x08011CB2: AIData +0x44's 0x20,
-- which every status reset clears first: the sea submerges him and doesn't hold him at a move's end).

local attach_points = require("@self/attach_points")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_chips = require("../weapons/charged_chip")
local darkwide = require("../../../chips/darkwide").darkwide
local soul_change = require("../../../rules/souls/change")
local soul_image = require("../../../rules/souls/image")
local soul_revert = require("../../../rules/souls/revert")

local form: FormDef = {
    soul = { family = "aqua" },
    change = soul_change.action,
    revert = soul_revert.action,
    -- ToadMan's image as MegaMan unites with him (rules/souls/image).
    image = soul_image.look { sprite = asset.sprite("toadman") },
    sprite = asset.sprite("toadsoul"),
    element = "aqua",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- Routine 0x1B (0x0800F520, empty: the charged chip is the chip with the bonus), the charge table's row 0x1B.
        a_charge = charged_chips.routine({ 20, 20, 20, 20, 20, 20 }),
        -- Routine 0x34 (0x0800FA5C): DarkWide (chip 0xC0), the charge table's row 0x34.
        chaos = chaos.weapon(darkwide, { 40, 60, 90, 120, 150, 40 }),
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
    -- The buster arm he raises in it (lib/arm, 0x080EBABE): its animation, the soul's.
    buster_arm = { anim = 12 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 12 of category 0).
    identity = new.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points,
    },
}

return { toadsoul = form }
