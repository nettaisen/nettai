--!strict
-- heatman: a link navi.

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local heatman = require("@self/chip")

return define.navi {
    id = "heatman",
    sprite = asset.sprite("heatman"),
    element = "fire",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("heatman-win"), lose = asset.banner("heatman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`): the Falzar ROM has no
    -- HeatMan face, and shows SpoutMan's.
    mugshot = asset.mugshot("heatman"),
    merge_height = 8,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("heatman-portrait"),
    },
    own_chip = { chip = heatman, code = "H" },
    chip_bonus = {
        family = "fire",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 0, 0, 0, 30, 30, 30, 30, 40, 40, 40, 50 },
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 100, float_shoes = true, mega_level = 5, giga_level = 1 },
    cross_hp = { 100, 150 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 800 },
        by_level = {
            { hp = 100, float_shoes = true },
            { hp = 150, float_shoes = true },
            { hp = 200, charge = 1, float_shoes = true },
            { hp = 250, charge = 1, float_shoes = true },
            { hp = 300, attack = 1, charge = 1, float_shoes = true },
            { hp = 350, attack = 1, charge = 1, float_shoes = true },
            { hp = 400, attack = 1, charge = 1, float_shoes = true },
            { hp = 450, attack = 1, charge = 1, custom_level = 1, float_shoes = true },
            { hp = 500, attack = 1, rapid = 1, charge = 1, custom_level = 1, float_shoes = true },
            { hp = 550, attack = 2, rapid = 1, charge = 1, custom_level = 1, float_shoes = true },
            { hp = 600, attack = 2, rapid = 1, charge = 1, custom_level = 1, float_shoes = true },
            { hp = 700, attack = 2, rapid = 1, charge = 1, custom_level = 1, float_shoes = true },
            { hp = 850, attack = 2, rapid = 1, charge = 1, custom_level = 1, mega_level = 1, float_shoes = true },
            { hp = 1000, attack = 2, rapid = 2, charge = 1, custom_level = 1, mega_level = 1, float_shoes = true },
            { hp = 1200, attack = 3, rapid = 2, charge = 2, custom_level = 1, mega_level = 1, float_shoes = true },
        },
    },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 1,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 1,
        ice = "large",
        -- What he wears over his body; a flinch restarts it too.
        parts = {
            body = {
                sprite = asset.sprite("heatman-overlay"),
                anims = 160,
                behind = { { 0x15, 0x17 }, { 0x22, 0x23 }, { 0x3B, 0x4F }, { 0x64, 0x8B }, { 0x9F, 0x9F } },
            },
        },
        overlay_hooks = { "death", "refresh", "flinch" },
        attach_points = {
            { 0, 8 },
            { 0, 8 },
            { 0, 8 },
            { 6, 53 },
            { 8, 55 },
            { 8, 55 },
            { 24, 0 },
            { 22, 22 },
            { 16, 22 },
            { 26, 22 },
            { 18, 23 },
            { 24, 23 },
            { 20, 22 },
            { 24, 22 },
            { 26, 23 },
            { 24, 22 },
            { 59, 25 },
            { 8, 16 },
            { 33, 24 },
            { 40, 24 },
            { 22, 24 },
            { 2, 24 },
            { 26, 22 },
            { 24, 23 },
            { 24, 21 },
            { 24, 23 },
            { 28, 22 },
            { 2, 16 },
            { 91, 14 },
            { 120, -5 },
            { -4, 28 },
            { 67, 16 },
            { 6, 8 },
            { 0, 0 },
        },
    },
}
