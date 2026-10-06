--!strict
-- EXE5's ToadMan (navi 12, NameID 0x18C), of Team Colonel: a team navi, which a player operates in MegaMan's place.
-- From EXE5's tables by navi: his battle sprite (08-0C, 0x0800DA65), his element (Aqua, 0x0800E634), his buster
-- bonus (0x0800F5AC), his move lag (0x0801D462's row), his netbattle banners (0x080090D8, 0x080090B0), his face in
-- the emotion window (0x0801971C on a Team Colonel ROM), and his identity: his actor record (0x08014C94: version 0,
-- a player, AI index 12) and his sprite's 30 attach points (category 8's table, 0x08015C1C, row 12).
--
-- His stats row (0x0801D55F, which 0x080111AA reads) gives him his weapons: the buster (routine 0, MegaMan's), the
-- dive (0x2E, ./charge: he goes under and comes up before an enemy) and an A charge (0x2C: an empty routine, as the
-- souls' charged chips'). His A charge is of his element's chips (0x0801090A, by navi after the souls: an Aqua chip
-- under 0x190 that is neither a dimming nor a dark chip), and a charged one hits twice as hard (0x080103D0). A team
-- navi has no forms. His own chip is S-Melody T (./chip: 0x08025EA0's pair for him).
--
-- He dives: his post-init hook (0x080F199C, the one entry of 0x080EB2A8 that isn't empty) sets the navi state that
-- lets a panel that submerges take him under (the sea), as ToadSoul's reset does MegaMan.

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local charged_chips = require("./megaman/weapons/charged_chip")
local s_melody = require("@self/chip")["s-melody"]

local navi: NaviDef = {
    sprite = asset.sprite("toadman"),
    element = "aqua",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("toadman-win"), lose = asset.banner("toadman-deleted") },
    mugshot = asset.mugshot("toadman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x22, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("toadman-portrait"),
    },
    own_chip = { chip = s_melody, code = "T" },
    own_chip_draws = true,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- The charge table's row 0x2C (0x0801CC24); Charge 5 reads on into row 0x2D's first entry.
        a_charge = charged_chips.routine({ 60, 60, 60, 60, 60, 140 }, "toadman/a-charge"),
    },
    -- The chips A charges, and what a charged one gains (its damage again).
    charged_chips = { family = "aqua", damaging = false, plain = true },
    charged_bonus = { damage = 0xFF },
    -- 0x080F199C
    post_init = function(navi: Object)
        navi:set_navi_state("dives", true)
    end,
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 300, mega_level = 5, giga_level = 1 },
    -- What the story gives him (0x08052E9C: the table 0x0804F960's row for his place in his team, Meddy's and his):
    -- his HP by the story's progress; his level, which his damage rows are read at, is that count up to 6.
    story = { hp = { 300, 300, 300, 300, 300, 300, 400, 500, 600 }, max_level = 6 },
    identity = new.identity {
        class = "player",
        version = 0,
        actor_type = "player",
        ai_index = 12,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 12,
        attach_points = {
            { 0, -4 },
            { 3, 0 },
            { 0, 0 },
            { 0, 45 },
            { 0, 34 },
            { 8, 40 },
            { 22, 0 },
            { 16, 18 },
            { 14, 16 },
            { 25, 16 },
            { 21, 15 },
            { 21, 16 },
            { 21, 16 },
            { 24, 15 },
            { 24, 16 },
            { 24, 16 },
            { 38, 17 },
            { 0, 14 },
            { 24, 18 },
            { 40, 16 },
            { 0, 16 },
            { 0, 15 },
            { 24, 17 },
            { 25, 17 },
            { 24, 16 },
            { 27, 16 },
            { 4, 6 },
            { 0, 14 },
            { 24, 25 },
            { 36, 18 },
        },
    },
}

return { toadman = navi }
