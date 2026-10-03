--!strict
-- ShadowSoul's B+Back (`shadowsoul/back`): the original's weapon routine 0x45 (0x0800FBEE) sets 50 damage, its own
-- counter byte, no bonus, the attack's variant 1, 30 ticks and a 50-tick wait before the next B+Back, and starts
-- action 0x3D (0x080EE9F4): the anti-damage stance (@common/megaman/anti_damage, BN6's AntiDmg program's), whose
-- caught hit turns into BN5's anti-damage counter (the roles' `anti_damage_counter`, or `anti_sword_counter`) with
-- that variant: the shuriken thrown at the nearest enemy ahead (@common/antidmg/counter).

local anti_damage = require("@common/megaman/anti_damage")

local back = {}

local DAMAGE, HIT_PARAM, TICKS, COOLDOWN = 50, 0x94, 30, 50

back.action = anti_damage.stance("bn5:shadowsoul/back/action")

-- 0x0800FBEE (AIAttackVars +3 1, which the counter's start, 0x0800E340, carries as its variant).
local function setup(navi: Object): Action
    navi.attack_damage = DAMAGE
    navi.hit_param = HIT_PARAM
    navi.extra = 0
    navi.attack_variant = 1
    local s = navi:action_state(back.action) :: { timer: number }
    s.timer = TICKS
    navi.attack_lockout = COOLDOWN
    return back.action
end

back.weapon = define.weapon {
    id = "bn5:shadowsoul/back",
    -- (A B+Back, which nothing charges: the charge table's row 0x45 (0x0801CD1E), Charge 5 reading on into row
    -- 0x46's first entry.)
    charge_ticks = { 60, 60, 60, 60, 60, 120 },
    setup = setup,
}

return back
