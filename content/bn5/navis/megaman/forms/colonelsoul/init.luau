--!strict
-- ColonelSoul (soul 7, Team Colonel's): BN5's Soul Unison with Colonel, for a Summon chip given up. MegaMan's sprite in
-- it is 00-07 (0x0800DA58[7]), his element Null (0x0800E634 by soul); he wears nothing over it (Colonel's image wears
-- his cape). Its weapons (0x0801CA1C's row 7, loaded by 0x0800DCA0): the buster (routine 0), the screen divide (0x14,
-- ./charge), routine 1 on A (the charged shot's: no chip charges in ColonelSoul, 0x0801090A), Chaos Unison's (0x3A:
-- DrkSonic); no B+Back (0xFF). His sword's arm is the blade's animation 14 (0x080EC038). Its status reset (0x08011C44)
-- arms its side's army: the obstacles on the other side's panels turn into its soldiers (objects/soldier, the
-- obstacle framework's step; bn5-map.md §15.11), which strike with the damage words it sets.

local attach_points = require("../attach_points")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_shot = require("../weapons/charged-shot")
local drksonic = require("../../../chips/drksonic")
local souls = require("../../../rules/souls")

return define.form {
    id = "colonelsoul",
    soul = { number = 7, family = "summon" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("colonelsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x3A (0x0800FA98): DrkSonic (chip 0xC6), the charge table's row 0x3A.
        chaos = chaos.weapon("colonelsoul/chaos", drksonic, { 40, 60, 90, 120, 150, 60 }),
    },
    -- 0x08011C44: the damage words (0x08011C50: 40 and 10 more per point of buster Attack plus one for the sword's
    -- soldier, 10 and 2 more for the gun's, each paralyzing with counter byte 0x94; 0x080CABF8), then the side armed
    -- (0x080CAC1E: the battle state's +0x5C bit 0x10 or 0x20). A status reset disarms it again (the engine's).
    reset = function(navi: Object)
        local level = battle.navi(navi.alliance).attack + 1
        obstacle.arm_soldiers(navi.alliance, 40 + 10 * level + 0x0094_4000, 10 + 2 * level + 0x0094_4000)
    end,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 0x16-0x1B, Team Colonel's
    -- souls'); in its Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("colonelsoul"),
        variant = { normal = asset.mugshot("colonelsoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 7 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[7],
    },
}
