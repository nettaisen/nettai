--!strict
-- Gregar Beast Over: one of MegaMan's forms.

local falzar_beast = require("../falzar-beast/form")
local gregar_beast_buster = require("../../weapons/gregar-beast-buster/weapon")
local beast_claw = require("../../weapons/beast-claw/weapon")
local forms = require("../../../../rules/forms/system")

return define.form {
    id = "gregar-beast-over",
    kind = "beast_over",
    -- What changes MegaMan into it (BN6's five sequences).
    change = forms.action,
    game = "gregar",
    -- (No hit breaks Beast Over; the original's routine, by the form's number, would drop
    -- either to the Falzar beast.)
    breaks_to = falzar_beast,
    sprite = asset.sprite("gregar-beast"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = gregar_beast_buster.weapon, alt_a_charge = beast_claw.weapon },
    -- Its face in the emotion window (`byte_801E700`): the Falzar ROM has no
    -- Gregar Beast Over face, and shows Falzar Beast Over's.
    mugshot = asset.mugshot("gregar-beast-over"),
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "super_armor", "lockon_marker", "invulnerable", "berserk" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 23,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 23 },
    identity = define.identity {
        class = "beast_over",
        version = 0,
        actor_type = "player",
        ai_index = 47,
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
