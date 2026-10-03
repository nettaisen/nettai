--!strict
-- ColonelSoul (soul 7, Team Colonel's): BN5's Soul Unison with Colonel, for a Summon chip given up. MegaMan's sprite in
-- it is 00-07 (0x0800DA58[7]), his element Null (0x0800E634 by soul); he wears nothing over it (Colonel's image wears
-- his cape). Its weapons (0x0801CA1C's row 7, loaded by 0x0800DCA0): the buster (routine 0), the screen divide (0x14,
-- ./charge), routine 1 on A (the charged shot's: no chip charges in ColonelSoul, 0x0801090A), Chaos Unison's (0x3A:
-- DrkSonic); no B+Back (0xFF). His sword's arm is the blade's animation 14 (0x080EC038).
--
-- Its status reset's routine (0x08011C44) arms its side's obstacles (the battle state's +0x5C bit by side, and two
-- damage words at 0x02034000 by side: 40 and 10 more per Attack point plus one, 10 and 2 more, paralyzing, counter
-- byte 0x94) for the obstacles' soldiers (0x080CA952); every status reset disarms them first (0x080CAC30). That
-- state is the obstacles' (BN5's obstacle framework's per-side state): `arm_obstacles` is a stub until it lands.

local attach_points = require("../../attach_points")
local buster = require("../../weapons/buster")
local chaos = require("../../weapons/chaos")
local charge = require("./charge")
local charged_shot = require("../../weapons/charged-shot")
local drksonic = require("../../../../chips/drksonic/chip")
local souls = require("../../../../rules/souls/system")

-- STUB (waits for BN5's obstacle framework's per-side arming slot): 0x08011C50's damage words and 0x080CAC1E's bit.
local function arm_obstacles(navi: Object) end

return define.form {
    id = "bn5:colonelsoul",
    kind = "soul",
    soul = { number = 7, family = "summon" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("bn5:colonelsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x3A (0x0800FA98): DrkSonic (chip 0xC6), the charge table's row 0x3A.
        chaos = chaos.weapon("bn5:colonelsoul/chaos", drksonic, { 40, 60, 90, 120, 150, 60 }),
    },
    reset = arm_obstacles,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 0x16-0x1B, Team Colonel's
    -- souls'); in its Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("bn5:colonelsoul"),
        variant = { normal = asset.mugshot("bn5:colonelsoul-chaos") },
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
