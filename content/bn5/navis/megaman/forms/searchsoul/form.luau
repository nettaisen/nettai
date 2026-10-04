--!strict
-- SearchSoul (soul 3, Team ProtoMan's): BN5's Soul Unison with SearchMan, for a Cursor chip given up. MegaMan's
-- sprite in it is 00-03 (0x0800DA58[3]), his element Null (0x0800E634 by soul); he wears nothing over it. Its weapons
-- (0x0801CA1C's row 3, loaded by 0x0800DCA0): the buster (routine 0), the scope's shots (8, ./charge), routine 1 on A
-- (the charged shot's: no chip charges in SearchSoul, 0x0801090A), Chaos Unison's (0x30: DarkCirc); no B+Back (0xFF).
-- Its status reset (0x08011C00) reveals the other side's invisible navis (./reveal); on a netbattle's console the
-- other side's HP shows over its navi (by MegaMan's attach point 3: drawn only, which the engine has not).

local attach_points = require("../../attach_points")
local buster = require("../../weapons/buster")
local chaos = require("../../weapons/chaos")
local charge = require("./charge")
local charged_shot = require("../../weapons/charged-shot")
local darkcirc = require("../../../../chips/darkcirc/chip")
local reveal = require("./reveal")
local souls = require("../../../../rules/souls/system")

return define.form {
    id = "bn5:searchsoul",
    soul = { number = 3, family = "cursor" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("bn5:searchsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x30 (0x0800FA34): DarkCirc (chip 0xBB), the charge table's row 0x30.
        chaos = chaos.weapon("bn5:searchsoul/chaos", darkcirc, { 40, 60, 90, 120, 150, 40 }),
    },
    reset = function(navi: Object)
        reveal.spawn(navi)
    end,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls'); in its
    -- Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("bn5:searchsoul"),
        variant = { normal = asset.mugshot("bn5:searchsoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 3 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[3],
    },
}
