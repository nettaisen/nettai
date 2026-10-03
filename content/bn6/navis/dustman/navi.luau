--!strict
-- dustman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local charge = require("./charge")
local dustman = require("./chip")

return define.navi {
    id = "bn6:dustman",
    sprite = asset.sprite("bn6:dustman"),
    element = "null",
    weakness = { "cursor" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("bn6:dustman-win"), lose = asset.banner("bn6:dustman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("bn6:dustman"),
    merge_height = 0,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("bn6:dustman-portrait"),
    },
    own_chip = { chip = dustman, code = "D" },
    chip_bonus = {
        family = "break",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10 },
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 250, super_armor = true, mega_level = 5, giga_level = 1 },
    cross_hp = { 150, 150 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 800 },
        by_level = {
            { hp = 100, super_armor = true },
            { hp = 150, super_armor = true },
            { hp = 200, rapid = 1, super_armor = true },
            { hp = 250, rapid = 1, super_armor = true },
            { hp = 300, rapid = 1, custom_level = 1, super_armor = true },
            { hp = 350, rapid = 1, custom_level = 1, super_armor = true },
            { hp = 400, rapid = 1, charge = 1, custom_level = 1, super_armor = true },
            { hp = 450, attack = 1, rapid = 1, charge = 1, custom_level = 1, super_armor = true },
            { hp = 500, attack = 1, rapid = 1, charge = 1, custom_level = 1, super_armor = true },
            { hp = 550, attack = 1, rapid = 2, charge = 2, custom_level = 2, super_armor = true },
            { hp = 600, attack = 1, rapid = 2, charge = 2, custom_level = 2, super_armor = true },
            { hp = 650, attack = 1, rapid = 2, charge = 2, custom_level = 2, super_armor = true },
            { hp = 800, attack = 2, rapid = 3, charge = 2, custom_level = 2, super_armor = true },
            { hp = 1000, attack = 2, rapid = 3, charge = 2, custom_level = 2, mega_level = 1, super_armor = true },
            { hp = 1200, attack = 2, rapid = 3, charge = 2, custom_level = 3, mega_level = 1, super_armor = true },
        },
    },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 10,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 10,
        ice = "large",
        attach_points = {
            { 2, 6 },
            { 2, 6 },
            { 2, 6 },
            { 0, 63 },
            { 0, 60 },
            { 8, 61 },
            { 29, 0 },
            { 23, 29 },
            { 16, 24 },
            { 30, 29 },
            { 24, 29 },
            { 25, 29 },
            { 27, 29 },
            { 29, 28 },
            { 32, 29 },
            { 27, 28 },
            { 66, 31 },
            { 0, 25 },
            { 39, 30 },
            { 43, 31 },
            { 27, 30 },
            { 0, 25 },
            { 32, 29 },
            { 32, 30 },
            { 28, 28 },
            { 29, 31 },
            { 30, 30 },
            { 0, 25 },
            { 106, 20 },
            { 118, -1 },
            { 0, 25 },
            { 78, 24 },
            { 0, 4 },
            { 0, 0 },
        },
    },
}
