--!strict
-- EXE5's KnightMan (navi 11, NameID 0x18B), of Team Colonel: a team navi, which a player operates in MegaMan's place.
-- From EXE5's tables by navi: his battle sprite (08-0B, 0x0800DA65), his element (none, 0x0800E634), his buster bonus
-- (0x0800F5AC), his move lag (10 ticks, 0x0801D462's row: the others' is 4), his netbattle banners (0x080090D8,
-- 0x080090B0), his face in the emotion window (0x0801971C on a Team Colonel ROM), and his identity: his actor record
-- (0x08014C94: version 0, a player, AI index 11) and his sprite's 30 attach points (category 8's table, 0x08015C1C,
-- row 11).
--
-- His stats row (0x0801D55F, which 0x080111AA reads) gives him SuperArmor, and his weapons: the buster (routine 0,
-- MegaMan's) and the wrecking ball's swing (0x10, ./charge); no B+Back. A team navi has no forms. His own chip is
-- KCrusher K (./chip: 0x08025EA0's pair for him), which throws his iron ball (./crusher).
--
-- He wears his ball and chain (his actor record's hook, 0x0800EE7A: the body overlays' row 1, sprite 10-47, in
-- front at every animation), which his deletion takes off (0x0800F008) and an animation change restarts
-- (0x0800F13C's entry for him). His attacks hide it while its ball is away and hold it so (./chain); his hook in the
-- paralysis, flinch and drag tables (0x080FBA20) lets go of that hold, so that an attack cut short leaves it
-- following him again.
--
-- His state table (0x080EADA8's row 11) has one more action, 7 (0x080FBD98: he hurls his ball up and away, attack
-- object #0x14, and it comes down on panels of the other side's), which nothing a player does starts (its chip,
-- 0x1A4, has no code): the operation battle's KnightMan's, which nettai doesn't build. So are the glow and the
-- dropped guard his actions start with there (the attack's variant 0).

local body_overlays = require("../lib/body_overlays")
local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local kcrusher = require("@self/chip")

return define.navi {
    id = "knightman",
    sprite = asset.sprite("knightman"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10 },
    banners = { win = asset.banner("knightman-win"), lose = asset.banner("knightman-deleted") },
    mugshot = asset.mugshot("knightman"),
    own_chip = { chip = kcrusher, code = "K" },
    own_chip_draws = true,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 500, super_armor = true, mega_level = 5, giga_level = 1 },
    -- What the story gives him (0x08052E9C: the table 0x0804F960's row for his place in his team, MagnetMan's and
    -- his): his HP by the story's progress; his level, which his damage rows are read at, is that count up to 6.
    story = { hp = { 400, 400, 500, 550, 600, 650, 700, 800, 900 }, max_level = 6 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 11,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 11,
        parts = { body = body_overlays.knightman },
        overlay_hooks = { "death", "refresh", "lets_go" },
        attach_points = {
            { 0, 16 },
            { 0, 36 },
            { 0, 24 },
            { 0, 75 },
            { 0, 75 },
            { 0, 75 },
            { 22, 0 },
            { 18, 45 },
            { 10, 43 },
            { 24, 43 },
            { 14, 45 },
            { 16, 45 },
            { 17, 45 },
            { 18, 43 },
            { 20, 44 },
            { 24, 43 },
            { 38, 45 },
            { 0, 36 },
            { 28, 43 },
            { 38, 44 },
            { 0, 44 },
            { 0, 44 },
            { 24, 45 },
            { 24, 45 },
            { 24, 43 },
            { 25, 44 },
            { 3, 35 },
            { 0, 36 },
            { 24, 25 },
            { 40, 44 },
        },
    },
}
