--!strict
-- The AntiDmg program's B+Back special (`megaman/anti-damage`): the
-- original's weapon routine 0x3D (`sub_80121BC`) and its stance, action
-- 0x5A (`sub_80EFE7C`). For 31 ticks the navi's
-- trap is armed (the navi state "trap_armed": a hit is caught as AntiDmg's
-- chip would, `sub_802CEF4`); a caught hit (requests 0x200 or 0x400) turns
-- the stance into the counter (`sub_80105F2`: AntiDmg's action 0x47, or
-- AntiSwrd's 0x48 for a sword hit) with the stance's 50 damage. The navi's
-- chips then wait 70 ticks.

local common = require("@common/megaman/anti_damage")

local anti_damage = {}

local DAMAGE, LOCKOUT, TICKS = 0x32, 0x46, 0x1E

-- The stance, action 0x5A (`sub_80EFE7C`: @common/megaman/anti_damage).
anti_damage.action = common.stance("bn6:megaman/anti-damage/action")

-- `sub_80121BC`.
local function setup(navi: Object): Action
    navi.attack_element = 0
    navi.attack_variant = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = DAMAGE
    local s = navi:action_state(anti_damage.action) :: { timer: number }
    s.timer = TICKS
    navi.attack_lockout = LOCKOUT
    return anti_damage.action
end

anti_damage.weapon = define.weapon {
    id = "bn6:megaman/anti-damage",
    -- (A B+Back special, which nothing charges: the charge table's row
    -- 0x3D by Charge 0 to 7, reading on into the next routine's row.)
    charge_ticks = { 0, 0, 0, 0, 0, 0, 0, 0 },
    setup = setup,
}

return anti_damage
