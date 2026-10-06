--!strict
-- ColonelSoul (soul 7, Team Colonel's): EXE5's Soul Unison with Colonel, for a Summon chip given up. MegaMan's sprite in
-- it is 00-07 (0x0800DA58[7]), his element Null (0x0800E634 by soul); he wears nothing over it (Colonel's image wears
-- his cape). Its weapons (0x0801CA1C's row 7, loaded by 0x0800DCA0): the buster (routine 0), the screen divide (0x14,
-- ./charge), routine 1 on A (the charged shot's: no chip charges in ColonelSoul, 0x0801090A), Chaos Unison's (0x3A:
-- DrkSonic); no B+Back (0xFF). His sword's arm is the blade's animation 14 (0x080EC038). Its status reset (0x08011C44)
-- arms its side's army: the obstacles on the other side's panels turn into its soldiers (objects/soldier, the
-- obstacle framework's step; exe5-map.md §15.11), which strike with the damage words it sets.

local arm_change = require("@self/arm_change")
local attach_points = require("@self/attach_points")
local body_overlays = require("../../../lib/body_overlays")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_shot = require("../weapons/charged_shot")
local drksonic = require("../../../chips/drksonic").drksonic
local soul_change = require("../../../rules/souls/change")
local soul_image = require("../../../rules/souls/image")
local soul_revert = require("../../../rules/souls/revert")

local form: FormDef = {
    soul = { family = "summon" },
    change = soul_change.action,
    revert = soul_revert.action,
    -- Colonel's image as MegaMan unites with him (rules/souls/image), in his cape (0x0800EDBC's routine for the soul).
    image = soul_image.wearing(asset.sprite("colonel"), body_overlays.colonel),
    -- What it adds to the custom screen (rules/souls gathers it, rules/souls/custom): Arm Change (./arm_change).
    custom = arm_change.custom,
    sprite = asset.sprite("colonelsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x3A (0x0800FA98): DrkSonic (chip 0xC6), the charge table's row 0x3A.
        chaos = chaos.weapon(drksonic, { 40, 60, 90, 120, 150, 60 }),
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
    -- The buster arm he raises in it (lib/arm, 0x080EBABE): its animation, the soul's.
    buster_arm = { anim = 7 },
    -- His sword's arm (lib/swords, 0x080EC038): the blade's animation.
    blade_anim = 14,
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 7 of category 0).
    identity = new.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points,
    },
}

return { colonelsoul = form }
