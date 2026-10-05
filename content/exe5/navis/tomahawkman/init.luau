--!strict
-- EXE5's TomahawkMan (navi 10, NameID 0x18A), of Team Colonel: a team navi, which a player operates in MegaMan's
-- place. From EXE5's tables by navi: his battle sprite (08-0A, 0x0800DA65), his element (Wood, 0x0800E634), his buster
-- bonus (3, 0x0800F5AC), his move lag (0x0801D462's row), his netbattle banners (0x080090D8, 0x080090B0), his face
-- in the emotion window (0x0801971C on a Team Colonel ROM), his palette step (2, 0x0801D737), and his identity: his
-- actor record (0x08014C94: version 0, a player, AI index 10) and his sprite's 30 attach points (category 8's
-- table, 0x08015C1C, row 10).
--
-- His stats row (0x0801D55F, which 0x080111AA reads) gives him SuperArmor and UnderShirt, and his weapons: the buster
-- (routine 0, MegaMan's), the thrown tomahawk (0x3C, ./charge) and an A charge (0x2B: an empty routine, as the souls'
-- charged chips'). His A charge is of his element's chips (0x0801090A, by navi after the souls: a Wood chip under
-- 0x190 that is neither a dimming nor a dark chip), and a charged one hits twice as hard (0x080103D0). A team navi
-- has no forms. His own chip is T-Swing T (./chip: 0x08025EA0's pair for him).

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local charged_chips = require("./megaman/weapons/charged-chip")
local t_swing = require("@self/chip")

return define.navi {
    id = "tomahawkman",
    sprite = asset.sprite("tomahawkman"),
    element = "wood",
    buster_bonus = 3,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("tomahawkman-win"), lose = asset.banner("tomahawkman-deleted") },
    mugshot = asset.mugshot("tomahawkman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x20, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("tomahawkman-portrait"),
    },
    palette_step = 2,
    own_chip = { chip = t_swing, code = "T" },
    own_chip_draws = true,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- The charge table's row 0x2B (0x0801CC1A); Charge 5 reads on into row 0x2C's first entry.
        a_charge = charged_chips.routine("tomahawkman/a-charge", { 60, 60, 60, 60, 60, 60 }),
    },
    -- The chips A charges, and what a charged one gains (its damage again).
    charged_chips = { family = "wood", damaging = false, plain = true },
    charged_bonus = { damage = 0xFF },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 400, super_armor = true, undershirt = true, mega_level = 5, giga_level = 1 },
    -- What the story gives him (0x08052E9C: the table 0x0804F960's row for his place in his team, NapalmMan's and
    -- his): his HP by the story's progress; his level, which his damage rows are read at, is that count up to 6.
    story = { hp = { 300, 300, 300, 300, 350, 400, 500, 600, 700 }, max_level = 6 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 10,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 10,
        attach_points = {
            { 8, 2 },
            { 0, 6 },
            { 0, 0 },
            { 8, 56 },
            { 8, 48 },
            { 14, 48 },
            { 22, 0 },
            { 12, 22 },
            { 14, 22 },
            { 22, 20 },
            { 16, 20 },
            { 16, 20 },
            { 16, 20 },
            { 20, 20 },
            { 20, 20 },
            { 21, 20 },
            { 34, 21 },
            { 8, 20 },
            { 26, 20 },
            { 38, 21 },
            { -1, 20 },
            { 8, 22 },
            { 23, 21 },
            { 23, 21 },
            { 22, 20 },
            { 24, 20 },
            { 2, 12 },
            { 8, 20 },
            { 24, 25 },
            { 36, 20 },
        },
    },
}
