--!strict
-- EXE5's NapalmMan (navi 4, NameID 0x184), of Team ProtoMan: a team navi (docs/design/exe5-map.md §15.16), from EXE5's
-- tables by navi as ProtoMan's are (navis/protoman): his battle sprite (08-04), his element (Fire), his buster bonus
-- and move lag, his banners, his face, his palette step (3), his actor record (version 0, a player, AI index 4) and
-- his sprite's attach points (category 8's row 4).
--
-- He wears his cannon (his actor record's hook, 0x0800EE8A: the body overlays' row 4, sprite 10-18), which his
-- deletion takes off (0x0800F016) and an animation change restarts (0x0800F13C's entry for him); his flinch,
-- paralysis and drag hooks are empty routines.
--
-- His weapons are his stats row's: his buster, a machine gun (0x44, ./buster); no charged attack and no B+Back; an
-- A charge (0x29: an empty routine, as the souls' charged chips'). His A charge is of his element's chips
-- (0x0801090A, by navi after the souls: a Fire chip under 0x190 that is neither a dimming nor a dark chip), and a
-- charged one hits twice as hard (0x080103D0). His own chip is Napalm N (./chip). His own action 9 (0x080F5FA4: two
-- bombs at a target his attack is given) is his attack in an auto battle, which no weapon or chip starts: it comes
-- with the gate navis' operation battle.

local body_overlays = require("../lib/body_overlays")
local buster = require("@self/buster")
local charged_chips = require("./megaman/weapons/charged_chip")
local napalm = require("@self/chip")

return define.navi {
    id = "napalmman",
    sprite = asset.sprite("napalmman"),
    element = "fire",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("napalmman-win"), lose = asset.banner("napalmman-deleted") },
    mugshot = asset.mugshot("napalmman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x1a, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("napalmman-portrait"),
    },
    palette_step = 3,
    own_chip = { chip = napalm, code = "N" },
    own_chip_draws = true,
    weapons = {
        buster = buster.weapon,
        -- The charge table's row 0x29 (0x0801CC06); Charge 5 reads on into row 0x2A's first entry.
        a_charge = charged_chips.routine({ 120, 120, 120, 120, 120, 60 }, "napalmman/a-charge"),
    },
    -- The chips A charges, and what a charged one gains (its damage again).
    charged_chips = { family = "fire", damaging = false, plain = true },
    charged_bonus = { damage = 0xFF },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 300, mega_level = 5, giga_level = 1 },
    -- What the story gives him (its table's row for him, 0x0804F960): his HP by its progress; his level is that
    -- progress up to 6.
    story = { hp = { 300, 300, 300, 300, 350, 400, 500, 600, 700 }, max_level = 6 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 4,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 4,
        parts = { body = body_overlays.napalmman },
        overlay_hooks = { "death", "refresh" },
        attach_points = {
            { -4, 16 },
            { 0, 14 },
            { 0, 10 },
            { 0, 64 },
            { 0, 64 },
            { 4, 64 },
            { 18, 0 },
            { 18, 40 },
            { 10, 34 },
            { 26, 38 },
            { 20, 38 },
            { 19, 39 },
            { 18, 38 },
            { 24, 37 },
            { 23, 39 },
            { 24, 38 },
            { 46, 32 },
            { 0, 28 },
            { 26, 39 },
            { 32, 36 },
            { -1, 39 },
            { 0, 32 },
            { 22, 40 },
            { 24, 40 },
            { 24, 38 },
            { 24, 38 },
            { 2, 30 },
            { 0, 30 },
            { 24, 25 },
            { 42, 32 },
        },
    },
}
