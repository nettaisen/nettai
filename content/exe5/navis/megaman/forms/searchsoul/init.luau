--!strict
-- SearchSoul (soul 3, Team ProtoMan's): EXE5's Soul Unison with SearchMan, for a Cursor chip given up. MegaMan's
-- sprite in it is 00-03 (0x0800DA58[3]), his element Null (0x0800E634 by soul); he wears nothing over it. Its weapons
-- (0x0801CA1C's row 3, loaded by 0x0800DCA0): the buster (routine 0), the scope's shots (8, ./charge), routine 1 on A
-- (the charged shot's: no chip charges in SearchSoul, 0x0801090A), Chaos Unison's (0x30: DarkCirc); no B+Back (0xFF).
-- Its status reset (0x08011C00) reveals the other side's invisible navis (./reveal); on a netbattle's console the
-- other side's HP shows over its navi (by MegaMan's attach point 3: drawn only, which the engine has not).

local attach_points = require("@self/attach_points")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_shot = require("../weapons/charged_shot")
local darkcirc = require("../../../chips/darkcirc").darkcirc
local reveal = require("@self/reveal")
local shuffle = require("@self/shuffle")
local soul_change = require("../../../rules/souls/change")
local soul_image = require("../../../rules/souls/image")
local soul_revert = require("../../../rules/souls/revert")

local form: FormDef = {
    soul = { family = "cursor" },
    change = soul_change.action,
    revert = soul_revert.action,
    -- SearchMan's image as MegaMan unites with him (rules/souls/image).
    image = soul_image.look { sprite = asset.sprite("searchman") },
    -- What it adds to the custom screen (rules/souls gathers it, rules/souls/custom): Shuffle (./shuffle).
    custom = shuffle.custom,
    sprite = asset.sprite("searchsoul"),
    element = "null",
    buster_bonus = 0,
    -- Its Chaos Unison (rules/souls/chaos_charge): the weapon an armed charge released in the cycle's window fires:
    -- routine 0x30 (0x0800FA34): DarkCirc (chip 0xBB), the charge table's row 0x30.
    chaos = { weapon = chaos.weapon(darkcirc, { 40, 60, 90, 120, 150, 40 }) },
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
    },
    reset = function(navi: Object)
        reveal.spawn(navi)
    end,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls'); in its
    -- Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("searchsoul"),
        variant = { normal = asset.mugshot("searchsoul-chaos") },
    },
    -- The buster arm he raises in it (lib/arm, 0x080EBABE): its animation, the soul's.
    buster_arm = { anim = 3 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 3 of category 0).
    identity = new.identity {
        class = "player",
        changes_form = true,
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points,
    },
}

return { searchsoul = form }
