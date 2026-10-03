--!strict
-- ColonelSoul (soul 7, Team Colonel's): BN5's Soul Unison with Colonel, for a Summon chip given up. MegaMan's sprite in
-- it is 00-07 (0x0800DA58[7]), his element Null (0x0800E634 by soul); he wears nothing over it (Colonel's image wears
-- his cape). Its weapons (0x0801CA1C's row 7, loaded by 0x0800DCA0): the buster (routine 0), the screen divide (0x14,
-- ./charge), routine 1 on A (the charged shot's: no chip charges in ColonelSoul, 0x0801090A), Chaos Unison's (0x3A:
-- DrkSonic); no B+Back (0xFF). His sword's arm is the blade's animation 14 (0x080EC038).
--
-- Its status reset's routine (0x08011C44) arms its side's soldiers (the obstacles' cannon and gun, 0x080CA952) with
-- two damage words: 40 and 10 more per Attack point plus one, and 10 and 2 more, both paralyzing, counter byte 0x94
-- (0x080CABF8, and the side's bit, 0x080CAC1E). Every status reset disarms them first (0x080CAC30). That state is
-- the obstacles' (`obstacle.arm_soldiers`, BN5's obstacle framework's): `arm_soldiers` below is a stub until it
-- lands.

local attach_points = require("../../attach_points")
local buster = require("../../weapons/buster")
local chaos = require("../../weapons/chaos")
local charge = require("./charge")
local charged_shot = require("../../weapons/charged-shot")
local drksonic = require("../../../../chips/drksonic/chip")
local souls = require("../../../../rules/souls/system")

-- STUB (waits for the obstacle framework's soldiers): `obstacle.arm_soldiers(side, cannon, gun)`.
local function arm_soldiers(side: number, cannon: number, gun: number) end

local HIT_PARAM = 0x94
-- The damage word's paralyzing bit.
local PARALYZE = 0x4000

-- 0x08011C44: the soldiers' damage words (0x08011C50, 0x0800FE5E by the navi's Attack).
local function arm_obstacles(navi: Object)
    local per = battle.navi(navi.alliance).attack + 1
    local cannon = 40 + 10 * per + HIT_PARAM * 0x10000 + PARALYZE
    local gun = 10 + 2 * per + HIT_PARAM * 0x10000 + PARALYZE
    arm_soldiers(navi.alliance, int.u32(cannon), int.u32(gun))
end

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
