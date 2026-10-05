--!strict
-- groundman: a link navi.

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local groundman = require("@self/chip")

return define.navi {
    id = "groundman",
    sprite = asset.sprite("groundman"),
    element = "null",
    weakness = { "cursor" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("groundman-win"), lose = asset.banner("groundman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("groundman"),
    merge_height = 0,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("groundman-portrait"),
    },
    own_chip = { chip = groundman, code = "G" },
    chip_bonus = {
        family = "break",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10 },
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 300, super_armor = true, mega_level = 5, giga_level = 1 },
    cross_hp = { 180, 180 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 800 },
        by_level = {
            { hp = 60, super_armor = true },
            { hp = 120, super_armor = true },
            { hp = 180, charge = 1, super_armor = true },
            { hp = 240, charge = 1, super_armor = true },
            { hp = 300, rapid = 1, charge = 1, mega_level = 1, super_armor = true },
            { hp = 360, rapid = 1, charge = 1, mega_level = 1, super_armor = true },
            { hp = 420, rapid = 1, charge = 1, mega_level = 1, super_armor = true },
            { hp = 490, attack = 1, rapid = 1, charge = 1, mega_level = 1, super_armor = true },
            { hp = 560, attack = 1, rapid = 1, charge = 1, mega_level = 1, super_armor = true },
            { hp = 630, attack = 1, rapid = 1, charge = 2, mega_level = 2, super_armor = true },
            { hp = 700, attack = 1, rapid = 1, charge = 2, mega_level = 2, super_armor = true },
            { hp = 800, attack = 2, rapid = 2, charge = 2, mega_level = 2, super_armor = true },
            { hp = 900, attack = 2, rapid = 2, charge = 2, custom_level = 1, mega_level = 2, super_armor = true },
            { hp = 1000, attack = 3, rapid = 2, charge = 2, custom_level = 1, mega_level = 2, super_armor = true },
            { hp = 1200, attack = 4, rapid = 2, charge = 2, custom_level = 1, mega_level = 2, super_armor = true },
        },
    },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 9,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 9,
        ice = "large",
        -- What he wears over his body.
        parts = {
            body = {
                sprite = asset.sprite("groundman-overlay"),
                anims = 60,
                behind = { { 0x00, 0x27 }, { 0x3B, 0x3B } },
            },
        },
        attach_points = {
            { -4, 4 },
            { -2, 22 },
            { -4, 4 },
            { 0, 64 },
            { 0, 64 },
            { 0, 58 },
            { 22, 0 },
            { 12, 33 },
            { 12, 33 },
            { 16, 32 },
            { 8, 32 },
            { 16, 32 },
            { 16, 32 },
            { 16, 32 },
            { 16, 33 },
            { 14, 31 },
            { 40, 33 },
            { -4, 16 },
            { 30, 33 },
            { 30, 32 },
            { 14, 33 },
            { -2, 22 },
            { 16, 32 },
            { 16, 32 },
            { 16, 32 },
            { 14, 33 },
            { 18, 31 },
            { -4, 16 },
            { 88, 16 },
            { 120, 0 },
            { -2, 22 },
            { 64, 22 },
            { -4, 8 },
            { 0, 0 },
        },
    },
}
