--!strict
-- The AntiDmg program's B+Back special, EXE5's and EXE6's (the same code): the weapon (EXE6's routine 0x3D,
-- `sub_80121BC`; EXE5's 0x21, 0x0800F938) and its stance (EXE6's action 0x5A, `sub_80EFE7C`; EXE5's 0x3D, 0x080EE9F4).
-- For 31 ticks the navi's trap is armed (the navi state "trap_armed": a hit is caught as AntiDmg's chip would, EXE6's
-- `sub_802CEF4`); a caught hit (requests 0x200 or 0x400) turns the stance into the counter (`sub_80105F2`: the game's
-- AntiDmg counter, or AntiSwrd's for a sword hit) with the stance's 50 damage and the attack's variant 0 (the
-- counter's aim, @exelib/antidmg/counter). The navi's chips then wait 70 ticks. A game makes its own (`make`, its ids);
-- the stance alone (`stance`) is EXE5's ShadowSoul's B+Back too, with its own setup.

export type State = {
    -- Ticks the trap stays armed (AIAttackVars+0x10).
    timer: number,
}

local anti_damage = {}

local DAMAGE, LOCKOUT, TICKS = 0x32, 0x46, 0x1E

-- The stance (`sub_80EFEAC`): the trap is armed until the timer runs out.
local function stance(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_navi_state("trap_armed", true)
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_navi_state("trap_armed", false)
        me:exit_attack()
    end
end

-- The stance action `id` (`sub_80EFE7C`): its setup gives it its ticks (`State.timer`), its damage, lockout and
-- variant.
function anti_damage.stance(id: string): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        -- `sub_80EFE7C`.
        update = function(me: Object, s: State)
            if me:request("anti_damage_triggered") or me:request("anti_sword_triggered") then
                me:start_stance_counter()
                return
            end
            local step = me.step
            if step == 0 then
                stance(me, s)
            else
                error(string.format("the AntiDmg stance's phase %#x reads past its table", step))
            end
        end,
    }
end

-- A game's AntiDmg weapon and its stance, by their ids (`weapon`, `action`), with its charge table's row (by Charge 0
-- on, reading on into the next routine's row: a B+Back special, which nothing charges).
function anti_damage.make(ids: { weapon: string, action: string, charge_ticks: { number } }): { action: Action, weapon: Weapon }
    local action = anti_damage.stance(ids.action)

    -- `sub_80121BC`.
    local function setup(navi: Object): Action
        navi.attack_element = 0
        navi.attack_variant = 0
        navi.charged = 0
        navi.extra = 0
        navi.hit_param = 0
        navi.attack_damage = DAMAGE
        local s = navi:action_state(action) :: State
        s.timer = TICKS
        navi.attack_lockout = LOCKOUT
        return action
    end

    local weapon = new.weapon {
        id = ids.weapon,
        charge_ticks = ids.charge_ticks,
        setup = setup,
    }
    return { action = action, weapon = weapon }
end

return anti_damage
