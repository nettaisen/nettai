--!strict
-- ThunderSoul's charged shot (`thundersoul/charged-shot`): weapon routine 0x58 (0x0800D298: 20 damage, elec, variant 3,
-- counter byte 0x8A, every other attack variable 0, its parameters 0) and the row shot it starts (action 0x47:
-- objects/row_shot, its row 0): the navi raises its hand (attachment row 0x1D, 0c-36) and an elec shot runs along its
-- row, 10 pixels a tick, paralyzing what it hits for 90 ticks (status 0x10).

local attachment = require("../../../../objects/attachment")
local row_shot = require("../../../../objects/row_shot")
local statuses = require("../../../../rules/status")

local charge = {}

charge.action = row_shot.make {
    id = "thundersoul/charged-shot",
    speed = 10,
    status = statuses.paralyze_90,
    sound = asset.sound("elec-shot"),
    sprite = asset.sprite("elec-shot"),
    hit_mod = 0,
    anim = 13,
    hand = attachment.look { sprite = asset.sprite("thundersoul-hand"), palette = 0 },
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
