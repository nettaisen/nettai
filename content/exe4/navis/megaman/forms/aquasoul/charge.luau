--!strict
-- AquaSoul's charged shot (`aquasoul/charged-shot`): weapon routine 0x23 (0x0800CF42: 20 aqua damage, counter byte
-- 0x94, variant 2, the attack parameter 0x0D, every other attack variable 0) and the spreaders' action it starts (0x1E,
-- chips/spreader/spreader) as Bubbler's (its variant 2: the Bubblers' gun, attachment row 0x0E, firing past the tenth
-- tick, from the ground) with bullet row 0x0D (0x080CF3DC: a bubble burst on the first thing it finds and the panel
-- behind it, hit modifier 3, effect row 0x25, sound 0x9F).

local attachment = require("../../../../objects/attachment")
local bullet = require("../../../../objects/bullet")
local regions = require("@exelib/regions")
local spreader = require("../../../../chips/spreader/spreader")

local charge = {}

-- The Bubblers' gun (attachment row 0x0E: 0c-1c), and the bubble burst (effect row 0x25) and its sound (0x9F).
local GUN = attachment.look { sprite = asset.sprite("bubbler"), palette = 0 }
local BURST = new.effect { sprite = asset.sprite("bubble-burst"), anim = 0, palette = 0 }
local SOUND = asset.sound("bubble-burst")
-- Bullet row 0x0D.
local SHOT = bullet.variant { region = regions.long, hit_mod = 3, effect = BURST, sound = SOUND }

charge.action = spreader.action { id = "aquasoul/charged-shot/action", gun = GUN, delay = 10, z = 0, shot = SHOT }

-- 0x0800CF42.
local function setup(navi: Object): Action
    navi.attack_element = 2
    navi.attack_variant = 2
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0x94
    navi.attack_damage = 20
    return charge.action
end

charge.weapon = new.weapon {
    id = "aquasoul/charged-shot",
    -- The charge table's row 0x23 (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x24's
    -- first entry.
    charge_ticks = { 30, 30, 30, 30, 30, 100 },
    setup = setup,
}

return charge
