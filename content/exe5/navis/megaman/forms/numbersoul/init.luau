--!strict
-- NumberSoul (soul 9, Team Colonel's): EXE5's Soul Unison with NumberMan, for a Plus chip given up. MegaMan's sprite in
-- it is 00-09 (0x0800DA58[9]), his element Null (0x0800E634 by soul); he wears its layer (./layer, row 1: his start
-- and end hooks, 0x0800F07C and 0x0800F0DE; NumberMan's image wears row 0). Its weapons (0x0801CA1C's row 9, loaded by
-- 0x0800DCA0): the buster (routine 0), the die (7, ./charge), routine 1 on A (the charged shot's: no chip charges in
-- NumberSoul, 0x0801090A), Chaos Unison's (0x32: DarkPlus); no B+Back (0xFF). Its damaging Null chips (dimming ones
-- too) deal 10 more (0x0800D0B8).

local attach_points = require("@self/attach_points")
local buster = require("../weapons/buster")
local chaos = require("../weapons/chaos")
local charge = require("@self/charge")
local charged_shot = require("../weapons/charged_shot")
local darkplus = require("../../../chips/darkplus")
local hand = require("@self/hand")
local layer = require("@self/layer")
local soul_change = require("../../../rules/souls/change")
local soul_image = require("../../../rules/souls/image")
local soul_revert = require("../../../rules/souls/revert")

-- His layer's row, and NumberMan's face's on his image.
local ROW, NUMBERMAN_FACE = 1, 0

return define.form {
    id = "numbersoul",
    soul = { family = "plus" },
    change = soul_change.action,
    revert = soul_revert.action,
    -- NumberMan's image as MegaMan unites with him (rules/souls/image): he wears his face, a layer (./layer, row 0)
    -- in the image's second related slot (0x0800EE6E), taken off as the image goes (0x0800EFFA).
    image = soul_image.look {
        sprite = asset.sprite("numberman"),
        put_on = function(image: Object)
            image.related2 = layer.spawn(image, NUMBERMAN_FACE)
        end,
        take_off = function(image: Object)
            layer.take_off(image.related2)
            image.related2 = nil
        end,
    },
    -- What it adds to the custom screen (rules/souls gathers it, rules/souls/custom): the hand of ten (./hand).
    custom = hand.custom,
    sprite = asset.sprite("numbersoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = charged_shot.weapon,
        -- Routine 0x32 (0x0800FA48): DarkPlus (chip 0xBE), the charge table's row 0x32.
        chaos = chaos.weapon(darkplus, { 40, 60, 90, 120, 150, 40 }),
    },
    chip_bonus = { family = "null", damage = 10, dimming_chips = true },
    -- His put-on and take-off routines (0x0800F038's and 0x0800F09C's row 9): his layer, in his second related slot
    -- (0x0800F07C); taken off (0x0800F0DE).
    put_on = function(navi: Object)
        navi.related2 = layer.spawn(navi, ROW)
    end,
    take_off = function(navi: Object)
        layer.take_off(navi.related2)
        navi.related2 = nil
    end,
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 0x16-0x1B, Team Colonel's
    -- souls'); in its Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on).
    mugshot = {
        normal = asset.mugshot("numbersoul"),
        variant = { normal = asset.mugshot("numbersoul-chaos") },
    },
    -- The buster arm he raises in it (lib/arm, 0x080EBABE): its animation, the soul's.
    buster_arm = { anim = 9 },
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 9 of category 0).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points,
    },
}
