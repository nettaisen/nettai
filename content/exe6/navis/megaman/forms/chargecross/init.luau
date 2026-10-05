--!strict
-- ChargeCross: one of MegaMan's forms.

local base = require("./base")
local cross_overlay = require("./cross_overlay")
local chargeman = require("../../chargeman")
local chargecross_beast = require("./chargecross-beast")
local buster = require("../weapons/buster")
local tackle = require("@self/tackle")
local charged_chip_bonus_29 = require("../weapons/charged-chip-bonus-29")
local forms = require("../../../rules/forms")

return define.form {
    id = "chargecross",
    kind = "cross",
    -- What changes MegaMan into it (EXE6's five sequences).
    change = forms.action,
    version = "gregar",
    cross_of = chargeman,
    beast = chargecross_beast,
    -- A weakness hit breaks the Cross: back to the base form.
    breaks_to = base,
    sprite = asset.sprite("megaman"),
    element = "fire",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = tackle.weapon, a_charge = charged_chip_bonus_29 },
    palette = 19,
    -- Its faces in the emotion window, and once Beast Out is spent
    -- (`byte_801E700`): the Falzar ROM has no ChargeCross face, and shows DustCross's.
    mugshot = { normal = asset.mugshot("chargecross"), tired = asset.mugshot("chargecross-tired") },
    -- What R shows in the Cross window.
    -- The chips A charges.
    charged_chips = { { family = "fire", damaging = true } },
    -- The A charge builds up the next Fire chip's damage, up to this much.
    fire_charge = 100,
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 5,
    -- The custom screen deals one more chip for each screen spent in it, up to three (the
    -- cross system's, `sub_802A49C`).
    extra_chips = true,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 5 },
    identity = define.identity {
        class = "cross",
        version = 0,
        actor_type = "player",
        ai_index = 29,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 29,
        ice = "medium",
        -- What it wears over MegaMan's body.
        parts = cross_overlay(asset.sprite("chargecross-overlay")),
        attach_points = {
            { 2, 0 },
            { 2, 0 },
            { 2, 0 },
            { 8, 48 },
            { 8, 48 },
            { 8, 48 },
            { 16, 0 },
            { 16, 24 },
            { 16, 24 },
            { 24, 24 },
            { 18, 24 },
            { 18, 24 },
            { 18, 24 },
            { 24, 24 },
            { 23, 25 },
            { 24, 24 },
            { 40, 26 },
            { 5, 11 },
            { 31, 25 },
            { 40, 26 },
            { 20, 25 },
            { 2, 16 },
            { 24, 24 },
            { 24, 25 },
            { 24, 23 },
            { 22, 25 },
            { 28, 25 },
            { 2, 18 },
            { 88, 16 },
            { 120, -4 },
            { -8, 16 },
            { 64, 16 },
            { 2, 0 },
            { 0, 0 },
        },
    },
}
