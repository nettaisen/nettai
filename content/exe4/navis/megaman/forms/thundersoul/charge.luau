--!strict
-- ThunderSoul's charged shot (`thundersoul/charged-shot`): weapon routine 0x58 (0x0800D298: 20 damage, elec, variant 3,
-- counter byte 0x8A, every other attack variable 0, its parameters 0) and the ring throw it starts (action 0x47:
-- lib/rings, holding the ring, attachment row 0x1D, 0c-36, in animation 13), of ring row 0 (objects/ring: the ring
-- 0c-37 along its row, 10 pixels a tick, paralyzing what it hits for 90 ticks, status 0x10; hit modifier 0; sound
-- 0x91): the zap ring's (navis/megaman/weapons/zap_ring).

local attachment = require("../../../../objects/attachment")
local ring = require("../../../../objects/ring")
local rings = require("../../../../lib/rings")
local statuses = require("../../../../rules/status")

local charge = {}

charge.action = rings.action {
    id = "thundersoul/charged-shot/action",
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

-- 0x0800D298.
local function setup(navi: Object): Action
    navi.attack_element = 3
    navi.attack_variant = 3
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    navi.hit_param = 0x8A
    navi.attack_damage = 20
    return charge.action
end

charge.weapon = new.weapon {
    id = "thundersoul/charged-shot",
    -- The charge table's row 0x58 (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x59's
    -- first entry.
    charge_ticks = { 90, 90, 90, 90, 90, 60 },
    setup = setup,
}

return charge
