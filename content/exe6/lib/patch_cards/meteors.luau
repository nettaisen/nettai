--!strict
-- The patch cards' meteors: MettFire's B button and Metrid's B+Back
-- special (the original's weapon routines 0x52 and 0x53, `sub_8012326` and
-- `loc_801232A`, which differ only in the drops: 1 and 3), and their
-- action 0x5C (`sub_80F0094`): the navi raises its arm, and on its third
-- tick a shower of Fire meteors (lib/instant/meteor_shower) starts dropping
-- them, 30 damage each, one on an opponent every 20 ticks. 16 ticks, then
-- 16 more before the navi is idle.

local meteor_shower = require("../instant/meteor_shower")

local meteors = {}

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10).
    timer: number,
    -- The meteors (the attack's second parameter byte).
    drops: number,
}

-- The phases, on the attack's step.
local CAST, RECOVER = 0x0, 0x4
-- The tick (timer value) the shower starts.
local SHOWER_AT = 0x0D
-- The attack: 30 damage, hit parameter 0x94.
local DAMAGE, HIT_PARAM = 0x1E, 0x94
-- Where the shower spawns: the registers the action leaves (`sub_80F00B0`'s
-- address in X, 0 in Y, BattleState's address in Z, what the attack
-- update's caller left). compat/games.toml has the X in the other ROMs.
local AT = Vec3.new(0x080F00B1, 0, 0x02034880)

-- `sub_80F00B0`: the arm up, the shower on the timer's 0xD; then the
-- recovery.
local function cast(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0x0F
        me.step_init = 4
    end
    if s.timer == SHOWER_AT then
        meteor_shower.spawn(me, AT, s.drops, me.attack_damage + me.hit_param * 0x10000)
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        s.timer = 0x0F
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80ECA0C`: 16 ticks, then back to idle.
local function recover(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:exit_attack()
    end
end

-- `sub_80F0094`.
local function update(me: Object, s: State)
    local step = me.step
    if step == CAST then
        cast(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("the meteors' phase %#x reads past their phase table", step))
    end
end

meteors.action = new.action {
    id = "patch-cards/meteors",
    state = { timer = "u16", drops = "u8" },
    update = update,
}

-- A card's meteors weapon: `drops` meteors (`sub_8012326`, `loc_801232A`),
-- with its charge row.
function meteors.weapon(spec: { id: string, drops: number, charge_ticks: { number } }): Weapon
    return new.weapon {
        id = spec.id,
        charge_ticks = spec.charge_ticks,
        setup = function(navi: Object): Action
            navi.attack_damage = DAMAGE
            navi.hit_param = HIT_PARAM
            local s = navi:action_state(meteors.action) :: State
            s.drops = spec.drops
            return meteors.action
        end,
    }
end

return meteors
