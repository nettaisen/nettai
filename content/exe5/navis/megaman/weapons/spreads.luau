--!strict
-- The patch cards' HeatS and BubSht (B↓HeatS, B↓BubSht): weapon routines 0x4A (0x0800FC5A) and 0x56 (0x0800FCFE),
-- the Spreaders' action (EXE5's 0x18, @exelib/spreadr/action) in its variants 1 and 2: Spreader's gun (the
-- attachment table's row 9, 0x080EC578), the bullet leaving after the count's 10 (0x080EC57C), from 16 pixels up and
-- from the ground (0x080EC580). HeatS fires the bullet's row 8, a Fire burst of 40 over the panel it finds and the
-- one behind; BubSht its row 4, an Aqua splash of 50.

local attachment = require("../../../objects/attachment")
local bullet = require("../../../objects/bullet")
local navi_arms = require("../../../lib/navi_arms")
local regions = require("@exelib/regions")
local spreader = require("@exelib/spreadr/action")

local spreads = {}

local LOOK: SpreaderLook = {
    sound = asset.sound("h-burst"),
    -- Held in the overlay, its animation by the holder's AI index (the arms' table).
    hold = attachment.holder(navi_arms.spreader),
    slot = "overlay",
    bullet_kind = bullet.kind,
    -- The navi fires in its animation 0xB.
    anim = 0xB,
}
local GUN = attachment.look { sprite = asset.sprite("spreader"), palette = 0, attach_point = 11 }
local FIRE_AFTER = 10

-- HeatS's bullet (row 8, its effect 0x22 and sound EXE6's): a burst over the panel found and the one behind it.
local HEAT_SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 0,
    effect = new.effect { sprite = asset.sprite("shell-burst"), anim = 0, palette = 0 },
    sound = asset.sound("target-move"),
}
-- BubSht's (row 4, its effect 0x1F and sound EXE6's): a splash over the panel found and the one behind it.
local BUBBLE_SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 0,
    effect = new.effect { sprite = asset.sprite("spout-splash"), anim = 0, palette = 0 },
    sound = asset.sound("spout-ball"),
}

local HEAT = spreader.action({
    id = "megaman/heat-shot/action",
    gun = GUN,
    z = 0x10_0000,
    bullet = HEAT_SHOT,
    fire_after = FIRE_AFTER,
}, LOOK)
local BUBBLE = spreader.action({
    id = "megaman/bubble-shot/action",
    gun = GUN,
    z = 0,
    bullet = BUBBLE_SHOT,
    fire_after = FIRE_AFTER,
}, LOOK)

local FIRE, AQUA = 1, 2

-- 0x0800FC5A: 40 damage, hit parameter 10, Fire (the attack's other bytes as they were).
spreads.heat = new.weapon {
    id = "megaman/heat-shot",
    -- The charge table's row 0x4A, by Charge 0 to 5: past 4 it reads on into row 0x4B's first entry.
    charge_ticks = { 60, 55, 50, 45, 40, 80 },
    setup = function(navi: Object): Action
        navi.attack_damage = 40
        navi.hit_param = 10
        navi.attack_element = FIRE
        return HEAT
    end,
}

-- 0x0800FCFE: 50 damage, hit parameter 10, Aqua.
spreads.bubble = new.weapon {
    id = "megaman/bubble-shot",
    -- The charge table's row 0x56, by Charge 0 to 5: past 4 it reads on into row 0x57's first entry.
    charge_ticks = { 60, 55, 50, 45, 40, 50 },
    setup = function(navi: Object): Action
        navi.attack_damage = 50
        navi.hit_param = 10
        navi.attack_element = AQUA
        return BUBBLE
    end,
}

return spreads
