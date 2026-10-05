--!strict
-- GyroSoul (soul 2, Team ProtoMan's): EXE5's Soul Unison with GyroMan, for a Wind chip given up. MegaMan's sprite in
-- it is 00-02 (0x0800DA58[2]), his element Null (0x0800E634 by soul), and he wears his propeller (0x0800F06C: the body
-- overlay row 2). Its weapons (0x0801CA1C's row 2, loaded by 0x0800DCA0): the buster (routine 0), the charged tornado
-- (9, ./charge), routine 1 on A (the charged shot's: no chip charges in GyroSoul, 0x0801090A), Chaos Unison's (0x39:
-- DarkTorn); no B+Back (0xFF). A Wind chip that isn't a dimming chip primes it (0x080102D2: AIData +0x0D, a sound),
-- and primed, the next damaging Wind or Null chip that isn't a dimming chip deals double (0x08010302), spending it;
-- while it is primed nothing else doubles a chip (0x0801026C: not Full Synchro nor anger); primed, his propeller
-- plays its spinning animations (MegaMan's per-form tick, 0x080F04CE: the overlay's animation 17 on, 0x080C451A).
-- Its status reset and a NaviCust change give him FloatShoes, a floating body and AirShoes (0x08011BE4, 0x08011D0C).

local attach_points = require("@self/attach_points")
local body_overlays = require("../../../lib/body_overlays")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_shot = require("../weapons/charged-shot")
local darktorn = require("../../../chips/darktorn")
local soul_change = require("../../../rules/souls/change")
local soul_image = require("../../../rules/souls/image")
local soul_revert = require("../../../rules/souls/revert")

-- 0x080F04CE in soul 2: the propeller's animation offset by the priming.
local PRIMED_ANIM = 17

return define.form {
    id = "gyrosoul",
    soul = { family = "wind" },
    change = soul_change.action,
    revert = soul_revert.action,
    -- GyroMan's image as MegaMan unites with him (rules/souls/image), in his propeller (0x0800EDBC's routine for the soul).
    image = soul_image.wearing(asset.sprite("gyroman"), "soul-image/gyroman", body_overlays.gyroman),
    sprite = asset.sprite("gyrosoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x39 (0x0800FA8E): DarkTorn (chip 0xC5), the charge table's row 0x39.
        chaos = chaos.weapon("gyrosoul/chaos", darktorn, { 40, 60, 90, 120, 150, 40 }),
    },
    status_reset = { "float_shoes", "floating_body", "air_shoes" },
    tick = function(navi: Object)
        navi:set_overlay_anim_offset(if navi.primed then PRIMED_ANIM else 0)
    end,
    priming = {
        by = { family = "wind" },
        sound = asset.sound("soul-primed"),
        doubles = { { family = "wind" }, { family = "null" } },
    },
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls'); in its
    -- Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("gyrosoul"),
        variant = { normal = asset.mugshot("gyrosoul-chaos") },
    },
    -- The buster arm he raises in it (lib/arm, 0x080EBABE): its animation, the soul's.
    buster_arm = { anim = 2 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 2 of category 0), wearing the
    -- propeller.
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points,
        parts = { body = body_overlays.gyrosoul },
    },
}
