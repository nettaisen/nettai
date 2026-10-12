--!strict
-- WoodSoul's charged shot (`woodsoul/charged-shot`): weapon routine 0x57 (0x0800D270: 20 wood damage, counter byte
-- 0x84, the attack parameters 0x00200000, every other attack variable 0) and the fan's action it starts (0x28,
-- chips/tornado/fan) as its variant 0: Tornado's fan (attachment row 0x0F: 0c-16 in palette 0) and one tornado two
-- panels ahead (0x080D3130, chips/tornado/tornado) of the parameters: its first 0, palette 1; no status; 0x20 ticks.

local attachment = require("../../../../objects/attachment")
local fan = require("../../../../chips/tornado/fan")
local tornado = require("../../../../chips/tornado/tornado")

local charge = {}

local FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 0 }
local LOOK = tornado.look { palette = 1, status = nil, ticks = 0x20 }

charge.action = fan.action(
    "woodsoul/charged-shot/action",
    FAN,
    function(me: Object, x: number, y: number, element: number, damage: number)
        tornado.spawn(me, x, y, element, damage, LOOK)
    end
)

-- 0x0800D270.
local function setup(navi: Object): Action
    navi.attack_element = 4
    navi.attack_variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0x84
    navi.attack_damage = 20
    return charge.action
end

charge.weapon = new.weapon {
    id = "woodsoul/charged-shot",
    -- The charge table's row 0x57 (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x58's
    -- first entry.
    charge_ticks = { 100, 100, 100, 100, 100, 90 },
    setup = setup,
}

return charge
