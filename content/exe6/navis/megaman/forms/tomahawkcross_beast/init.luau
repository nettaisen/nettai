--!strict
-- TomahawkCross Beast: one of MegaMan's forms.

local tomahawkman = require("../../tomahawkman").tomahawkman
local falzar_beast = require("./falzar_beast")["falzar-beast"]
local falzar_beast_buster = require("../weapons/falzar_beast_buster")
local charged_chip_bonus = require("../weapons/charged_chip_bonus")
local throw = require("@self/throw")
local forms = require("../../../rules/forms")

local form: FormDef = {
    kind = "cross_beast",
    -- What changes MegaMan into it (EXE6's five sequences).
    change = forms.action,
    version = "falzar",
    cross_of = tomahawkman,
    -- A weakness hit breaks the Cross: back to the Beast.
    breaks_to = falzar_beast,
    sprite = asset.sprite("tomahawkcross-beast"),
    element = "wood",
    buster_bonus = 0,
    weapons = { buster = falzar_beast_buster.weapon, a_charge = charged_chip_bonus, alt_a_charge = throw.weapon },
    mugshot = asset.mugshot("tomahawkcross-beast"),
    -- Beast Out: its damaging Null chips' bonus, where the family's doesn't apply.
    null_bonus = 30,
    -- The chips A charges.
    charged_chips = { { family = "wood", damaging = true }, { family = "null", damaging = false } },
    -- A charged Wood chip hits twice as hard.
    charge_doubles = { element = "wood" },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "clear_statuses", "air_shoes", "float_shoes", "floating_body", "target_marker" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 19,
    -- The target marker shows, its afterimages stay, and a Null chip's A charge times by the
    -- alternative routine (`sub_80E1566`, `sub_80E341E`, `sub_8012F62`).
    traits = { "status_immune", "shows_target_marker", "afterimages_stay", "alt_charge_time" },
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 19 },
    identity = new.identity {
        class = "cross_beast",
        version = 0,
        actor_type = "player",
        ai_index = 43,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 13,
        ice = "medium",
        -- The beast's head, in the Cross's palette.
        parts = { beast_head = { sprite = asset.sprite("beast-head"), palette = 4 } },
        attach_points = {
            { 4, 16 },
            { 4, 24 },
            { 4, 16 },
            { 9, 64 },
            { 9, 64 },
            { 9, 64 },
            { 16, 0 },
            { 16, 37 },
            { 16, 38 },
            { 24, 38 },
            { 17, 38 },
            { 17, 38 },
            { 18, 38 },
            { 22, 38 },
            { 22, 39 },
            { 24, 37 },
            { 40, 39 },
            { 9, 23 },
            { 29, 40 },
            { 40, 39 },
            { 18, 39 },
            { 4, 32 },
            { 22, 38 },
            { 22, 39 },
            { 24, 39 },
            { 20, 39 },
            { 25, 38 },
            { 4, 32 },
            { 88, 28 },
            { 114, 8 },
            { -8, 28 },
            { 65, 34 },
            { 4, 10 },
            { 4, 8 },
        },
    },
}

return { ["tomahawkcross-beast"] = form }
