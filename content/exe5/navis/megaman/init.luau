--!strict
-- EXE5's MegaMan (navi 0, NameID 0x180), from EXE5's tables: his battle sprite (08-00, 0x0800DA65 by navi), his
-- element (0x0800E634), his buster bonus (0x0800F5AC), his move lag (4 after a move, `sub_8010332`'s
-- counterpart 0x0800E0BE: MegaMan's isn't by variant), his netbattle banners (0x080090D8, 0x080090B0), his
-- weapons (weapons/: the buster and the charged shot, EXE6's actions with EXE5's setups), and his identity: his
-- actor record (0x08014C94: version 0, a player, AI index 0) and his sprite's 30 attach points (./attach_points). EXE5 has no weakness table (EXE6's `byte_80108D1`).
-- On sea, his Aqua chips with damage (dimming chips too) hit 30 harder and spend the sea under him, unless his soul
-- gives a bonus of its own (0x0800D0A6: NumberSoul's, NapalmSoul's; the panel turns Normal at 0x080100B0).
--
-- Not yet (docs/design/exe5-map.md §15.7): the held offset and the custom screen's no-running message.

local attach_points = require("@self/attach_points")
local buster = require("@self/weapons/buster")
local charged_shot = require("@self/weapons/charged-shot")
local colonelsoul = require("@self/forms/colonelsoul")
local gyrosoul = require("@self/forms/gyrosoul")
local knightsoul = require("@self/forms/knightsoul")
local magnetsoul = require("@self/forms/magnetsoul")
local meddysoul = require("@self/forms/meddysoul")
local napalmsoul = require("@self/forms/napalmsoul")
local numbersoul = require("@self/forms/numbersoul")
local protosoul = require("@self/forms/protosoul")
local searchsoul = require("@self/forms/searchsoul")
local shadowsoul = require("@self/forms/shadowsoul")
local toadsoul = require("@self/forms/toadsoul")
local tomahawksoul = require("@self/forms/tomahawksoul")

return define.navi {
    id = "megaman",
    sprite = asset.sprite("megaman-navi"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("megaman-win"), lose = asset.banner("megaman-deleted") },
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    panel_bonus = { panel = "sea", family = "aqua", damage = 30 },
    -- His fresh stats, as a navi's stats start (0x080111BA from the table 0x0801D55F: his row, EXE6's):
    -- the stats a match's side starts him at.
    fresh = { hp = 100, mega_level = 5, giga_level = 1 },
    -- His souls, which the custom screen's soul button offers (rules/souls), in the original's order of the navis he
    -- unites with (0x0873BBC8's names: compat's records.toml has each form's number): the order of the soul button's
    -- icons in a pack, where a frontend finds a soul's icon by its place here, and the place the round's souls given
    -- are counted by (rules/souls/custom's `used_bit`).
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
        -- The Full Synchro aura's animation: EXE6's rule (`sub_80C4C52`: the AI index), which EXE5 hasn't (its aura
        -- is its own, to come with EXE5's emotions); the engine asks every player for one.
        aura_anim = 0,
        -- He wears what his soul does: his death hook takes it off (0x0800EFCC, his record's category 2), and an
        -- animation change (0x0800F11C by AI index 0), a flinch and a drag (0x080F0500) restart it.
        overlay_hooks = { "death", "refresh", "flinch", "drag" },
        attach_points = attach_points,
    },
}
