--!strict
-- ShadowSoul (soul 8, Team Colonel's): BN5's Soul Unison with ShadowMan, for an Invisible chip given up. MegaMan's
-- sprite in it is 00-08 (0x0800DA58[8]), his element Null (0x0800E634 by soul); he wears nothing over it, floats
-- (his status reset's and a NaviCust change's FloatShoes and floating body, 0x08011C80, 0x08011D32) and ends a move
-- without lag (0x0800E0D2). Its weapons (0x0801CA1C's row 8, loaded by 0x0800DCA0): the buster (routine 0),
-- LongSwrd's slash (0x17, ./charge), Sword chips charged with A (0x18: any Sword chip but a dimming or dark one,
-- 0x0801090A; no bonus, 0x080103D0, but a charged sword warps behind the enemy first, lib/swords), the anti-damage
-- stance on B+Back (0x45, ./back). His sword's arm is the blade's animation 15 (0x080EC038).
--
-- Not yet: Chaos Unison's weapon (routine 0x31, 0x0800FA3E: DarkInvs, chip 0xBD, the charge table's row 0x31:
-- 40, 60, 90, 120, 150 and 40), which waits for DarkInvs's port.

local attach_points = require("../../attach_points")
local back = require("./back")
local buster = require("../../weapons/buster")
local charge = require("./charge")
local charged_chips = require("../../weapons/charged-chip")
local souls = require("../../../../rules/souls/system")

return define.form {
    id = "bn5:shadowsoul",
    kind = "soul",
    soul = { number = 8, family = "invisible" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("bn5:shadowsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        -- Routine 0x18 (0x0800F520, empty: the charged chip is the chip with its charge), the charge table's row 0x18.
        a_charge = charged_chips.routine("bn5:shadowsoul/a-charge", { 30, 30, 30, 30, 30, 70 }),
        back_special = back.weapon,
    },
    charged_chips = { { family = "sword", damaging = false, plain = true } },
    status_reset = { "float_shoes", "floating_body" },
    move_lag = 0,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 0x16-0x1B, Team Colonel's
    -- souls'); in its Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("bn5:shadowsoul"),
        variant = { normal = asset.mugshot("bn5:shadowsoul-chaos") },
    },
    buster_arm = { anim = 0 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 8 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points[8],
    },
}
