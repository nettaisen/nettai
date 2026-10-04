--!strict
-- GyroSoul (soul 2, Team ProtoMan's): BN5's Soul Unison with GyroMan, for a Wind chip given up. MegaMan's sprite in
-- it is 00-02 (0x0800DA58[2]), his element Null (0x0800E634 by soul), and he wears his propeller (0x0800F06C: the body
-- overlay row 2). Its weapons (0x0801CA1C's row 2, loaded by 0x0800DCA0): the buster (routine 0), the charged tornado
-- (9, ./charge), routine 1 on A (the charged shot's: no chip charges in GyroSoul, 0x0801090A), Chaos Unison's (0x39:
-- DarkTorn); no B+Back (0xFF). A Wind chip that isn't a dimming chip primes it (0x080102D2: AIData +0x0D, a sound),
-- and primed, the next damaging Wind or Null chip that isn't a dimming chip deals double (0x08010302), spending it;
-- while it is primed nothing else doubles a chip (0x0801026C: not Full Synchro nor anger). Its status reset and a
-- NaviCust change give him FloatShoes, a floating body and AirShoes (0x08011BE4, 0x08011D0C).

local attach_points = require("../../attach_points")
local body_overlays = require("../../../../lib/body_overlays")
local buster = require("../../weapons/buster")
local chaos = require("../../weapons/chaos")
local charge = require("./charge")
local charged_shot = require("../../weapons/charged-shot")
local darktorn = require("../../../../chips/darktorn/chip")
local souls = require("../../../../rules/souls/system")

return define.form {
    id = "bn5:gyrosoul",
    soul = { number = 2, family = "wind" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("bn5:gyrosoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x39 (0x0800FA8E): DarkTorn (chip 0xC5), the charge table's row 0x39.
        chaos = chaos.weapon("bn5:gyrosoul/chaos", darktorn, { 40, 60, 90, 120, 150, 40 }),
    },
    status_reset = { "float_shoes", "floating_body", "air_shoes" },
    priming = {
        by = { family = "wind" },
        sound = asset.sound("bn5:soul-primed"),
        doubles = { { family = "wind" }, { family = "null" } },
    },
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls'); in its
    -- Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("bn5:gyrosoul"),
        variant = { normal = asset.mugshot("bn5:gyrosoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 2 of category 0), wearing the
    -- propeller.
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[2],
        parts = { body = body_overlays.gyrosoul },
    },
}
