--!strict
-- ShadowMan's B+Back (`shadowman/back`): the original's weapon routine 0x0D (0x0800F704) takes the damage his level
-- gives (the damage rows' row 14, lib/navi_level: his stats row's 50 at +0x48 isn't read), his counter byte, no bonus,
-- the attack's variant 1, 30 ticks and a 50-tick wait before his next chip, and starts action 0x3D (0x080EE9F4): the
-- anti-damage stance (@exelib/navicust/anti-damage, the AntiDmg program's), whose caught hit turns into EXE5's
-- anti-damage counter (the roles' `anti_damage_counter`, or `anti_sword_counter`) with that variant: a shuriken at
-- the nearest enemy ahead (@exelib/antidmg/counter). ShadowSoul's (navis/megaman/forms/shadowsoul/back) is the same
-- stance with 50 damage.

local anti_damage = require("@exelib/navicust/anti-damage")
local navi_level = require("../../lib/navi_level")

local back = {}

local HIT_PARAM, TICKS, LOCKOUT = 0x94, 30, 50
-- The damage rows' row 14, by his level.
local DAMAGE: { number } = { 60, 60, 60, 80, 80, 100, 100 }

back.action = anti_damage.stance("shadowman/back/action")

-- 0x0800F704 (AIAttackVars +3 1, which the counter's start, 0x0800E340, carries as its variant).
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.extra = 0
    navi.attack_variant = 1
    local s = navi:action_state(back.action) :: { timer: number }
    s.timer = TICKS
    navi.attack_lockout = LOCKOUT
    return back.action
end

back.weapon = define.weapon {
    id = "shadowman/back",
    -- (A B+Back, which nothing charges: the charge table's row 0x0D (0x0801CAEE), Charge 5 reading on into row
    -- 0x0E's first entry.)
    charge_ticks = { 0, 0, 0, 0, 0, 70 },
    setup = setup,
}

return back
