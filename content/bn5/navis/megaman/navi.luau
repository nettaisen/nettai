--!strict
-- BN5's MegaMan (navi 0, NameID 0x180), from BN5's tables: his battle sprite (08-00, 0x0800DA65 by navi), his
-- element (0x0800E634), his buster bonus (0x0800F5AC), his move lag (4 after a move, `sub_8010332`'s
-- counterpart 0x0800E0BE: MegaMan's isn't by variant), his netbattle banners (0x080090D8, 0x080090B0), his
-- weapons (weapons/: the buster and the charged shot, BN6's actions with BN5's setups), and his identity: his
-- actor record (0x08014C94: version 0, a player, AI index 0) and his sprite's 30 attach points (./attach_points). BN5 has no weakness table (BN6's `byte_80108D1`).
-- On sea, his Aqua chips with damage (dimming chips too) hit 30 harder and spend the sea under him, unless his soul
-- gives a bonus of its own (0x0800D0A6: NumberSoul's, NapalmSoul's; the panel turns Normal at 0x080100B0).
--
-- Not yet (docs/design/bn5-map.md §15.7): the held offset and the custom screen's no-running message.

local attach_points = require("./attach_points")
local buster = require("./weapons/buster")
local charged_shot = require("./weapons/charged-shot")
local colonelsoul = require("./forms/colonelsoul/form")
local gyrosoul = require("./forms/gyrosoul/form")
local knightsoul = require("./forms/knightsoul/form")
local magnetsoul = require("./forms/magnetsoul/form")
local meddysoul = require("./forms/meddysoul/form")
local napalmsoul = require("./forms/napalmsoul/form")
local numbersoul = require("./forms/numbersoul/form")
local protosoul = require("./forms/protosoul/form")
local searchsoul = require("./forms/searchsoul/form")
local shadowsoul = require("./forms/shadowsoul/form")
local toadsoul = require("./forms/toadsoul/form")
local tomahawksoul = require("./forms/tomahawksoul/form")

return define.navi {
    id = "megaman",
    sprite = asset.sprite("megaman-navi"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("megaman-win"), lose = asset.banner("megaman-deleted") },
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    panel_bonus = { panel = "sea", family = "aqua", damage = 30 },
    -- His fresh stats, as a navi's stats start (0x080111BA from the table 0x0801D55F: his row, BN6's):
    -- the stats a match's side starts him at.
    fresh = { hp = 100, mega_level = 5, giga_level = 1 },
    -- His souls, which the custom screen's soul button offers (rules/souls).
    forms = {
        souls = {
            protosoul,
            gyrosoul,
            searchsoul,
            napalmsoul,
            magnetsoul,
            meddysoul,
            colonelsoul,
            shadowsoul,
            numbersoul,
            tomahawksoul,
            knightsoul,
            toadsoul,
        },
    },
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        -- The Full Synchro aura's animation: BN6's rule (`sub_80C4C52`: the AI index), which BN5 hasn't (its aura
        -- is its own, to come with BN5's emotions); the engine asks every player for one.
        aura_anim = 0,
        -- He wears what his soul does: his death hook takes it off (0x0800EFCC, his record's category 2), and an
        -- animation change (0x0800F11C by AI index 0), a flinch and a drag (0x080F0500) restart it.
        overlay_hooks = { "death", "refresh", "flinch", "drag" },
        attach_points = attach_points[0],
    },
}
