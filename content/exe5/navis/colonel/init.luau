--!strict
-- EXE5's Colonel (navi 7, NameID 0x187), Team Colonel's leader: a team navi (docs/design/exe5-map.md §15.16), from
-- EXE5's tables by navi as ProtoMan's are (navis/protoman): his battle sprite (08-07), his element, buster bonus and
-- move lag, his banners, his face, his actor record (version 0, a player, AI index 7) and his sprite's attach points
-- (category 8's row 7). His sprite's palettes go by two (0x0801D737).
--
-- His weapons are his stats row's: the buster (routine 0, MegaMan's) and the Screen Divide (0x46, ./charge); no B+Back.
-- His own chip is C-Cannon C (./chip). He wears his cape (his actor record's hook, 0x0800EE5E: the body overlays' row
-- 3, sprite 10-1E, in front at every animation), which his deletion takes off (0x0800EFEC) and an animation change
-- (0x0800F13C's entry for him) or a flinch (0x080F88AA, which tests for it first) restarts. (His flinch hook first
-- clears his object's +0x60, a word nothing else of his touches: his routines, 0x080F8600 to 0x080F9000, access it
-- nowhere else, and neither the shared actions nor the player's routines write a navi's. It is 0 whenever he
-- flinches, so the clear has nothing to port.)

local buster = require("./megaman/weapons/buster")
local c_cannon = require("@self/chip")
local charge = require("@self/charge")

return define.navi {
    id = "colonel",
    sprite = asset.sprite("colonel"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("colonel-win"), lose = asset.banner("colonel-deleted") },
    mugshot = asset.mugshot("colonel"),
    own_chip = { chip = c_cannon, code = "C" },
    own_chip_draws = true,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 200, mega_level = 5, giga_level = 1 },
    -- What the story gives him (its table's row for him, Team Colonel's 0x0804F968): his HP by its progress; his level
    -- is that progress up to 6.
    story = { hp = { 200, 300, 350, 400, 450, 500, 600, 700, 800 }, max_level = 6 },
    palette_step = 2,
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 7,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 7,
        parts = { body = { sprite = asset.sprite("colonel-cape"), anims = 41 } },
        overlay_hooks = { "death", "refresh", "flinch_checked" },
        attach_points = {
            { 0, 16 },
            { 0, 14 },
            { 0, 12 },
            { 0, 68 },
            { 0, 64 },
            { 4, 68 },
            { 18, 0 },
            { 24, 35 },
            { 21, 32 },
            { 32, 33 },
            { 27, 33 },
            { 27, 33 },
            { 28, 33 },
            { 30, 31 },
            { 31, 33 },
            { 32, 32 },
            { 50, 34 },
            { 0, 30 },
            { 34, 36 },
            { 46, 34 },
            { 4, 33 },
            { 0, 32 },
            { 31, 34 },
            { 32, 33 },
            { 31, 35 },
            { 32, 35 },
            { 2, 26 },
            { 0, 30 },
            { 24, 25 },
            { 46, 34 },
        },
    },
}
