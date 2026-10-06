--!strict
-- ElecCross: one of MegaMan's forms.

local base = require("./base").base
local cross_overlay = require("./cross_overlay")
local elecman = require("../../elecman").elecman
local eleccross_beast = require("./eleccross_beast")["eleccross-beast"]
local buster = require("../weapons/buster")
local charge = require("@self/charge")
local a_charge = require("@self/a_charge")
local forms = require("../../../rules/forms")

local form: FormDef = {
    kind = "cross",
    -- What changes MegaMan into it (EXE6's five sequences).
    change = forms.action,
    version = "gregar",
    cross_of = elecman,
    beast = eleccross_beast,
    -- A weakness hit breaks the Cross: back to the base form.
    breaks_to = base,
    sprite = asset.sprite("megaman"),
    element = "elec",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = a_charge, alt_a_charge = a_charge },
    palette = 7,
    -- Its faces in the emotion window, and once Beast Out is spent
    -- (`byte_801E700`): the Falzar ROM has no ElecCross face, and shows TomahawkCross's.
    mugshot = { normal = asset.mugshot("eleccross"), tired = asset.mugshot("eleccross-tired") },
    -- What R shows in the Cross window.
    -- The damage it adds to its family's damaging chips.
    chip_bonus = { family = "elec", damage = 50 },
    -- The chips A charges.
    charged_chips = { { family = "null", damaging = true } },
    -- A charged chip paralyzes.
    charged_bonus = { paralyzes = true },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 2,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 2 },
    identity = new.identity {
        class = "player",
        changes_form = true,
        breaks_on_weakness = true,
        version = 0,
        actor_type = "player",
        ai_index = 26,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 26,
        ice = "medium",
        -- What it wears over MegaMan's body.
        parts = cross_overlay(asset.sprite("eleccross-overlay")),
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

return { eleccross = form }
