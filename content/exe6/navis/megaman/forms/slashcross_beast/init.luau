--!strict
-- SlashCross Beast: one of MegaMan's forms.

local slashman = require("../../slashman").slashman
local gregar_beast = require("./gregar_beast")["gregar-beast"]
local gregar_beast_buster = require("../weapons/gregar_beast_buster")
local slash_a_charge = require("../weapons/slash_a_charge")
local lunge = require("@self/lunge")
local forms = require("../../../rules/forms")

local form: FormDef = {
    kind = "cross_beast",
    -- What changes MegaMan into it (EXE6's five sequences).
    change = forms.action,
    version = "gregar",
    cross_of = slashman,
    -- A weakness hit breaks the Cross: back to the Beast.
    breaks_to = gregar_beast,
    sprite = asset.sprite("slashcross-beast"),
    element = "null",
    weakness = { "break" },
    buster_bonus = 0,
    weapons = { buster = gregar_beast_buster.weapon, a_charge = slash_a_charge.weapon, alt_a_charge = lunge.weapon },
    -- Its face in the emotion window (`byte_801E700`): the Falzar ROM has no
    -- SlashCross Beast face, and shows TenguCross Beast's.
    mugshot = asset.mugshot("slashcross-beast"),
    -- The damage it adds to its family's damaging chips.
    chip_bonus = { family = "sword", damage = 50 },
    -- Beast Out: its damaging Null chips' bonus, where the family's doesn't apply.
    null_bonus = 30,
    -- The chips A charges.
    charged_chips = { { family = "sword", damaging = true, element_swords = true }, { family = "null", damaging = false } },
    -- A charged chip hits 40 harder.
    charged_bonus = { damage = 40 },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "super_armor", "target_marker", "invulnerable", "berserk" },
    -- After a NaviCust change only the SuperArmor comes back (`sub_801469C`).
    navicust_refresh = { "super_armor" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 15,
    -- The target marker shows, its afterimages stay, and a Null chip's A charge times by the
    -- alternative routine (`sub_80E1566`, `sub_80E341E`, `sub_8012F62`).
    traits = { "shows_target_marker", "afterimages_stay", "alt_charge_time" },
    -- Its charged sword runs inside the Beast Out rush (rules/beast's, `sub_800FB54`).
    charged_sword_rush = true,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 15, palette_add = 7 },
    identity = new.identity {
        class = "cross_beast",
        version = 0,
        actor_type = "player",
        ai_index = 39,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 12,
        ice = "medium",
        attach_points = {
            { 6, 0 },
            { 6, 0 },
            { 6, 0 },
            { 16, 44 },
            { 16, 44 },
            { 16, 44 },
            { 24, 0 },
            { 27, 17 },
            { 24, 17 },
            { 38, 18 },
            { 31, 18 },
            { 31, 18 },
            { 29, 17 },
            { 35, 17 },
            { 35, 19 },
            { 35, 17 },
            { 51, 20 },
            { 8, 10 },
            { 41, 20 },
            { 48, 19 },
            { 32, 19 },
            { 6, 16 },
            { 35, 18 },
            { 32, 18 },
            { 32, 16 },
            { 32, 19 },
            { 36, 17 },
            { 8, 16 },
            { 104, 8 },
            { 124, -10 },
            { -8, 24 },
            { 80, 14 },
            { 6, 0 },
            { 0, 4 },
        },
    },
}

return { ["slashcross-beast"] = form }
