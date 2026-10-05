--!strict
-- tomahawkman: a link navi.

local buster = require("./megaman/weapons/buster")
local a_charge = require("./megaman/weapons/charged-chip-bonus-1f")
local charge = require("@self/charge")
local tomahawkman = require("@self/chip")

return define.navi {
    id = "tomahawkman",
    sprite = asset.sprite("tomahawkman"),
    element = "wood",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("tomahawkman-win"), lose = asset.banner("tomahawkman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("tomahawkman"),
    merge_height = 0,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("tomahawkman-portrait"),
    },
    own_chip = { chip = tomahawkman, code = "T" },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = a_charge },
    fresh = { hp = 200, super_armor = true, mega_level = 5, giga_level = 1 },
    cross_hp = { 100, 150 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 800 },
        by_level = {
            { hp = 100, super_armor = true },
            { hp = 150, super_armor = true },
            { hp = 200, rapid = 1, mega_level = 1, super_armor = true },
            { hp = 250, rapid = 1, mega_level = 1, super_armor = true },
            { hp = 300, attack = 1, rapid = 1, mega_level = 1, super_armor = true },
            { hp = 350, attack = 1, rapid = 1, mega_level = 1, super_armor = true },
            { hp = 400, attack = 1, rapid = 1, mega_level = 1, super_armor = true },
            { hp = 450, attack = 1, rapid = 1, mega_level = 1, super_armor = true },
            { hp = 500, attack = 1, rapid = 1, mega_level = 1, super_armor = true },
            { hp = 550, attack = 2, rapid = 1, charge = 1, mega_level = 1, super_armor = true },
            { hp = 600, attack = 2, rapid = 1, charge = 1, mega_level = 1, super_armor = true },
            { hp = 650, attack = 2, rapid = 1, charge = 1, custom_level = 1, mega_level = 2, super_armor = true },
            { hp = 700, attack = 3, rapid = 1, charge = 1, custom_level = 1, mega_level = 2, super_armor = true },
            { hp = 800, attack = 3, rapid = 2, charge = 2, custom_level = 1, mega_level = 2, super_armor = true },
            { hp = 1000, attack = 4, rapid = 2, charge = 2, custom_level = 1, mega_level = 2, super_armor = true },
        },
    },
    -- The chips A charges, from a navi level on, and those a charge doubles.
    charged_chips = { family = "wood", from_level = 11 },
    charge_doubles = { element = "wood" },
    -- A hit's status doesn't take.
    traits = { "status_immune" },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 7,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 7,
        ice = "medium",
        attach_points = {
            { 0, 0 },
            { 4, 8 },
            { 0, 0 },
            { 10, 54 },
            { 10, 50 },
            { 10, 46 },
            { 24, 0 },
            { 13, 21 },
            { 13, 21 },
            { 22, 20 },
            { 16, 20 },
            { 18, 20 },
            { 18, 20 },
            { 22, 21 },
            { 22, 21 },
            { 20, 20 },
            { 34, 22 },
            { 6, 8 },
            { 31, 21 },
            { 34, 21 },
            { 18, 21 },
            { 4, 22 },
            { 22, 20 },
            { 22, 21 },
            { 16, 20 },
            { 22, 21 },
            { 22, 20 },
            { 6, 8 },
            { 88, 11 },
            { 120, -8 },
            { 4, 22 },
            { 64, 16 },
            { 0, 0 },
            { 8, 8 },
        },
    },
}
