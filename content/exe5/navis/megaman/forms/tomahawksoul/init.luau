--!strict
-- TomahawkSoul (soul 10, Team Colonel's): EXE5's Soul Unison with TomahawkMan, for a Wood chip given up. MegaMan's
-- sprite in it is 00-0A (0x0800DA58[10]), his element Wood (0x0800E634 by soul); he wears nothing over it, and his
-- change shakes the cameras (rules/souls/change). Its weapons (0x0801CA1C's row 10, loaded by 0x0800DCA0): the buster
-- (routine 0), the tomahawk's swing (0x1A, ./charge), routine 1 on A (the charged shot's: no chip charges in
-- TomahawkSoul, 0x0801090A), Chaos Unison's (0x33: DrkLance); no B+Back (0xFF). Its status reset turns the field to
-- grass (0x08011C98: ./grass), and its damaging Wood chips but dimming ones deal double while he stands on grass,
-- which the use turns normal (0x0801032A, 0x0800FFD6).

local attach_points = require("@self/attach_points")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_shot = require("../weapons/charged-shot")
local drklance = require("../../../chips/drklance")
local grass = require("@self/grass")
local soul_change = require("../../../rules/souls/change")
local soul_image = require("../../../rules/souls/image")
local soul_revert = require("../../../rules/souls/revert")

return define.form {
    id = "tomahawksoul",
    soul = { family = "wood" },
    change = soul_change.action,
    revert = soul_revert.action,
    -- TomahawkMan's image as MegaMan unites with him (rules/souls/image).
    image = soul_image.look { sprite = asset.sprite("tomahawkman") },
    sprite = asset.sprite("tomahawksoul"),
    element = "wood",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x33 (0x0800FA52): DrkLance (chip 0xBF), the charge table's row 0x33.
        chaos = chaos.weapon("tomahawksoul/chaos", drklance, { 40, 60, 90, 120, 150, 40 }),
    },
    grass_doubles = { family = "wood" },
    reset = function(navi: Object)
        grass.spawn(navi)
    end,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 0x16-0x1B, Team Colonel's
    -- souls'); in its Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("tomahawksoul"),
        variant = { normal = asset.mugshot("tomahawksoul-chaos") },
    },
    -- The buster arm he raises in it (lib/arm, 0x080EBABE): its animation, the soul's.
    buster_arm = { anim = 10 },
    -- MegaMan's emerging in it shakes the cameras (rules/souls/change, 0x0801212C).
    emerge_shake = true,
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 10 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points,
    },
}
