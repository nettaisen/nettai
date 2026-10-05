--!strict
-- MeddySoul (soul 6, Team ProtoMan's): EXE5's Soul Unison with Meddy, for a Recovery chip given up. MegaMan's sprite in
-- it is 00-06 (0x0800DA58[6]), his element Null (0x0800E634 by soul); he wears nothing over it. Its weapons
-- (0x0801CA1C's row 6, loaded by 0x0800DCA0): the buster (routine 0), Meddy's capsule (0xE, ./charge), routine 1 on A
-- (the charged shot's: no chip charges in MeddySoul, 0x0801090A), Chaos Unison's (0x36: DrkRecov), its charge cycle
-- the chaos cycle's row 2 whatever the chaos level (0x080106BC); no B+Back (0xFF).

local attach_points = require("../attach_points")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_shot = require("../weapons/charged-shot")
local drkrecov = require("../../../chips/drkrecov")
local souls = require("../../../rules/souls")

return define.form {
    id = "meddysoul",
    soul = { number = 6, family = "recovery", chaos_cycle = 2 },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("meddysoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x36 (0x0800FA70): DrkRecov (chip 0xC2), the charge table's row 0x36.
        chaos = chaos.weapon("meddysoul/chaos", drkrecov, { 70, 90, 110, 130, 150, 70 }),
    },
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls'); in its
    -- Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("meddysoul"),
        variant = { normal = asset.mugshot("meddysoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 6 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[6],
    },
}
