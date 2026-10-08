--!strict
-- MegaMan's zap ring (`megaman/zap-ring`, weapon routine 0x04, 0x0800CD02: patch card 81's charged shot): Elec, its
-- variant 3, damage by the navi's weapon level (AIData +8, NaviStats +0x0B: 30, 40, 50 by 0x0800CD28), hit parameter
-- 10, its parameters 0; the ring throw (action 0x47, lib/rings) in animation 13, holding the ring (attachment row 0x1D,
-- 0c-36), of ring row 0 (0x080DAFB0: 10 pixels a tick, paralysis for 90 ticks, sound 0x91, the ring 0c-37, hit modifier
-- 0). Its charge row (4): 210, 190, 170, 150, 130, and Charge 5 row 5's first, 180.

local attachment = require("../../../objects/attachment")
local ring = require("../../../objects/ring")
local rings = require("../../../lib/rings")
local statuses = require("../../../rules/status")

local zap_ring = {}

-- The damage by weapon level (0x0800CD28).
local DAMAGE: { number } = { 30, 40, 50 }
local ELEC, VARIANT, HIT_PARAM = 3, 3, 10

local ACTION = rings.action {
    id = "megaman/zap-ring/action",
    anim = 13,
    held = attachment.look { sprite = asset.sprite("zap-ring-arm"), palette = 0 },
    ring = ring.variant {
        sprite = asset.sprite("zap-ring"),
        speed = 10,
        status = statuses.paralyze_90,
        sound = asset.sound("zap-ring"),
        hit_mod = 0,
        sand = nil,
    },
}

-- 0x0800CD02.
local function setup(navi: Object): Action
    navi.attack_element = ELEC
    navi.attack_variant = VARIANT
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    navi.hit_param = HIT_PARAM
    local level = battle.navi(navi.alliance).weapon_level
    local damage = DAMAGE[level + 1]
    if damage == nil then
        error(string.format("the zap ring's damage reads past 0x0800CD28 at weapon level %d", level))
    end
    navi.attack_damage = damage
    return ACTION
end

zap_ring.weapon = new.weapon {
    id = "megaman/zap-ring",
    charge_ticks = { 210, 190, 170, 150, 130, 180 },
    setup = setup,
}

return zap_ring
