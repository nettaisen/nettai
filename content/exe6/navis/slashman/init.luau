--!strict
-- slashman: a link navi.

local by_level = require("../rules/by_level")
local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local slashman = require("@self/chip")

return define.navi {
    id = "slashman",
    sprite = asset.sprite("slashman"),
    element = "null",
    weakness = { "break" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("slashman-win"), lose = asset.banner("slashman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`): the Falzar ROM has no
    -- SlashMan face, and shows TenguMan's.
    mugshot = asset.mugshot("slashman"),
    version = "gregar",
    merge_height = 4,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("slashman-portrait"),
    },
    own_chip = { chip = slashman, code = "S" },
    chip_bonus = {
        family = "sword",
        dimming_chips = false,
        damage = by_level.row({ 0, 0, 0, 0, 0, 0, 0, 30, 30, 30, 30, 40, 40, 40, 50 }),
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 150, mega_level = 5, giga_level = 1 },
    cross_hp = { 150, 150 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 800 },
        by_level = {
            { hp = 40 },
            { hp = 80 },
            { hp = 120, rapid = 1 },
            { hp = 160, rapid = 1 },
            { hp = 200, attack = 1, rapid = 1 },
            { hp = 240, attack = 1, rapid = 2 },
            { hp = 280, attack = 1, rapid = 2 },
            { hp = 320, attack = 1, rapid = 2, custom_level = 1 },
            { hp = 360, attack = 1, rapid = 3, custom_level = 1 },
            { hp = 400, attack = 1, rapid = 3, custom_level = 1 },
            { hp = 500, attack = 2, rapid = 3, custom_level = 1 },
            { hp = 600, attack = 2, rapid = 3, custom_level = 1 },
            { hp = 700, attack = 2, rapid = 3, custom_level = 2 },
            { hp = 800, attack = 2, rapid = 4, charge = 1, custom_level = 2, mega_level = 1 },
            { hp = 1000, attack = 2, rapid = 4, charge = 1, custom_level = 2, mega_level = 1 },
        },
    },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 3,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 3,
        ice = "medium",
        attach_points = {
            { 0, 0 },
            { 3, 8 },
            { 0, 0 },
            { 8, 50 },
            { 8, 48 },
            { 8, 48 },
            { 26, 0 },
            { 24, 26 },
            { 16, 25 },
            { 32, 24 },
            { 26, 24 },
            { 28, 24 },
            { 30, 26 },
            { 32, 24 },
            { 34, 26 },
            { 32, 24 },
            { 48, 26 },
            { 4, 10 },
            { 36, 26 },
            { 44, 26 },
            { 28, 26 },
            { 4, 18 },
            { 32, 24 },
            { 32, 25 },
            { 24, 24 },
            { 34, 26 },
            { 35, 24 },
            { 0, 18 },
            { 98, 16 },
            { 124, -4 },
            { -10, 22 },
            { 64, 22 },
            { 3, 0 },
            { 0, 10 },
        },
    },
}
