--!strict
-- spoutman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local a_charge = require("../megaman/weapons/charged-chip-bonus-20/weapon")
local charge = require("./charge")
local spoutman = require("./chip")

return define.navi {
    id = "spoutman",
    name = "SpoutMan",
    sprite = asset.sprite("spoutman"),
    element = "aqua",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("spoutman-win"), lose = asset.banner("spoutman-deleted") },
    merge_height = 0,
    run_message = { 19, 5 },
    own_chip = { chip = spoutman, code = "A" },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = a_charge },
    fresh = { hp = 100, first_barrier = 0, mega_level = 5, giga_level = 1 },
    cross_hp = { 150, 150 },
    -- The chips A charges, from a navi level on, and those a charge doubles.
    charged_chips = { family = "aqua", from_level = 11 },
    charge_doubles = { element = "aqua" },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 6,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 6,
        ice = "medium",
        -- What he wears while he stands.
        parts = { idle = { sprite = asset.sprite("idle-overlay") } },
        attach_points = {
            { 0, -4 },
            { 0, 0 },
            { 0, -4 },
            { 3, 42 },
            { 0, 38 },
            { 4, 38 },
            { 16, 0 },
            { 14, 14 },
            { 8, 16 },
            { 22, 13 },
            { 16, 12 },
            { 16, 12 },
            { 16, 14 },
            { 20, 13 },
            { 21, 13 },
            { 22, 12 },
            { 32, 14 },
            { 0, 8 },
            { 30, 14 },
            { 32, 16 },
            { 18, 14 },
            { 0, 16 },
            { 22, 12 },
            { 20, 13 },
            { 16, 16 },
            { 20, 13 },
            { 22, 13 },
            { 0, 8 },
            { 88, 4 },
            { 120, -16 },
            { -8, 16 },
            { 64, 8 },
            { 0, 0 },
            { 0, 0 },
        },
    },
}
