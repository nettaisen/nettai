--!strict
-- EXE5's MagnetMan (navi 5, NameID 0x185), of Team ProtoMan: a team navi (docs/design/exe5-map.md §15.16), from EXE5's
-- tables by navi as ProtoMan's are (navis/protoman): his battle sprite (08-05), his element (Elec), his buster bonus
-- and move lag, his banners, his face, his palette step (2), his actor record (version 0, a player, AI index 5) and
-- his sprite's attach points (category 8's row 5). His stats row gives him SuperArmor, FloatShoes and AirShoes. He
-- wears nothing, and none of the per-actor hooks is his.
--
-- His weapons are his stats row's: the buster (routine 0, MegaMan's), the magnet missile (0x26, ./charge), no
-- B+Back, and an A charge (0x2A: an empty routine, as the souls' charged chips'). His A charge is of his element's
-- chips (0x0801090A, by navi after the souls: an Elec chip under 0x190 that is neither a dimming nor a dark chip),
-- and a charged one hits twice as hard (0x080103D0). His own chip is NSTackle M (./chip).

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local charged_chips = require("./megaman/weapons/charged_chip")
local nstackle = require("@self/chip").nstackle

local navi: NaviDef = {
    sprite = asset.sprite("magnetman"),
    element = "elec",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("magnetman-win"), lose = asset.banner("magnetman-deleted") },
    mugshot = asset.mugshot("magnetman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x1b, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("magnetman-portrait"),
    },
    palette_step = 2,
    own_chip = { chip = nstackle, code = "M" },
    own_chip_draws = true,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- The charge table's row 0x2A (0x0801CC10); Charge 5 reads on into row 0x2B's first entry.
        a_charge = charged_chips.routine({ 60, 60, 60, 60, 60, 60 }, "magnetman/a-charge"),
    },
    -- The chips A charges, and what a charged one gains (its damage again).
    charged_chips = { family = "elec", damaging = false, plain = true },
    charged_bonus = { damage = 0xFF },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 400, super_armor = true, float_shoes = true, air_shoes = true, mega_level = 5, giga_level = 1 },
    -- What the story gives him (its table's row for him, 0x0804F960): his HP by its progress; his level is that
    -- progress up to 6.
    story = { hp = { 400, 400, 500, 550, 600, 650, 700, 800, 900 }, max_level = 6 },
    identity = new.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 5,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 5,
        attach_points = {
            { 0, 24 },
            { -3, 30 },
            { 0, 22 },
            { 0, 80 },
            { 0, 80 },
            { 5, 80 },
            { 20, 0 },
            { 20, 47 },
            { 18, 47 },
            { 25, 45 },
            { 16, 46 },
            { 22, 46 },
            { 21, 46 },
            { 24, 44 },
            { 24, 45 },
            { 27, 45 },
            { 53, 47 },
            { 0, 38 },
            { 32, 45 },
            { 42, 46 },
            { 4, 46 },
            { 0, 40 },
            { 30, 46 },
            { 30, 46 },
            { 26, 45 },
            { 31, 45 },
            { 6, 35 },
            { 0, 38 },
            { 24, 25 },
            { 46, 46 },
        },
    },
}

return { magnetman = navi }
