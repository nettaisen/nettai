--!strict
-- AntiDmg's counter, as EXE5 and EXE6 share it (EXE6's navi action 0x47,
-- `sub_80EE90C`; EXE5's 0x47): each game makes its own (`counter.action_at`:
-- its key, its shuriken's kind, its sounds and its puff). When
-- an AntiDmg trap (or the AntiDmg program's stance) catches a hit, the
-- navi vanishes in a puff and throws a shuriken (./shuriken) at an enemy,
-- by the attack's variant (`attack_variant`): a random one for 0, the nearest
-- ahead, its own row first, for 1, none for any other. A trap chip's catch
-- leaves it 0, the stance its weapon's: EXE6's AntiDmg program 0, EXE5's
-- ShadowSoul 1. The ruleset starts it by role (`actions.anti_damage_counter`,
-- a game's rules/roles.luau).

local regions = require("../regions")
local shuriken = require("./shuriken")

local counter = {}

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10).
    timer: number,
}

local STATE = { timer = "u16" }

-- A game's counter: its key, its shuriken's kind, the sounds of its
-- vanishing and its throw, and the puff as it vanishes.
export type Spec = {
    id: string,
    shuriken: Kind,
    vanish_sound: Sound,
    throw_sound: Sound,
    puff: Effect,
}

local VANISH_ANIM = 4
-- The shuriken's hit modifier, and the height it is thrown from.
local HIT_MOD = 1
local HEIGHT = 0xA0_0000
-- An enemy's body (`byte_8016290`, by side).
local ENEMY_BODY: { [number]: number } = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
-- The rows `sub_8016218` looks at, by the navi's row (`byte_8016298`).
local ROWS: { [number]: { number } } = { [1] = { 1, 2, 3 }, [2] = { 2, 1, 3 }, [3] = { 3, 1, 2 } }

local function set_step(me: Object, step: number)
    me.step, me.step_init = step, 0
end

-- The way a shuriken flies from the navi to (x, y): the navi's side,
-- turned when the target isn't ahead.
local function flip_toward(me: Object, x: number): number
    local behind = if me.panel_x - x >= 0 then 1 else 0
    return bit32.bxor(me.alliance, behind)
end

-- `sub_8016004`: one of the other side's combatants (in slot order),
-- picked by a draw.
local function random_enemy(me: Object): (number?, number?)
    local panels = {}
    for _, o in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
        table.insert(panels, bit32.bor(o.panel_x, bit32.lshift(o.panel_y, 4)))
    end
    if #panels == 0 then
        return nil, nil
    end
    local b = panels[battle.rng_positive() % #panels + 1]
    return bit32.band(b, 7), bit32.rshift(b, 4)
end

-- `sub_8016218`: the nearest enemy body ahead, column by column, the
-- navi's own row first.
local function nearest_enemy(me: Object): (number?, number?)
    local rows = ROWS[me.panel_y]
    if not rows then
        error(string.format("sub_8016218 reads past byte_8016298 for row %d", me.panel_y))
    end
    local body = ENEMY_BODY[me.alliance]
    local x = me.panel_x + me.facing
    while field.valid(x, 2) do
        for _, y in rows do
            if bit32.btest(field.flags(x, y), body) then
                return x, y
            end
        end
        x += me.facing
    end
    return nil, nil
end

-- `sub_80EE92C`: stand still on the panel (reserved), uncollidable; the
-- side's trap record goes; 4 ticks later, a puff, and on.
local function vanish(me: Object, s: State, spec: Spec)
    if me.step_init == 0 then
        me:set_animation(VANISH_ANIM)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me:set_status("sliding", false)
        me:set_status("moving", true)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me.collision:set_region(nil)
        battle.clear_linked(me.alliance)
        s.timer = 3
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t >= 0 then
        return
    end
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), spec.puff)
    battle.play_sound(spec.vanish_sound)
    set_step(me, 4)
end

-- `sub_80EE996`: hidden, throw the shuriken at the target the variant
-- picks (none: no throw), then 31 ticks.
local function throw(me: Object, s: State, spec: Spec)
    me.visible = false
    if me.step_init == 0 then
        local variant = me.attack_variant
        local x, y
        if variant == 0 then
            x, y = random_enemy(me)
        elseif variant == 1 then
            x, y = nearest_enemy(me)
        end
        if x and y then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            shuriken.spawn(spec.shuriken, me, x, y, me.attack_element, HIT_MOD, damage, flip_toward(me, x), HEIGHT)
        end
        battle.play_sound(spec.throw_sound)
        s.timer = 0x1E
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        set_step(me, 8)
    end
end

-- `sub_80EE9F2`: back, its trap request done; 6 ticks, then idle.
local function back(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0)
        me.collision:set_region(regions.single)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me:set_request("anti_damage_triggered", false)
        me:set_status("moving", false)
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me:exit_attack()
    end
end

-- A game's counter.
function counter.action_at(spec: Spec): Action
    return define.action {
        id = spec.id,
        state = STATE,
        -- `sub_80EE90C`, by phase.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                vanish(me, s, spec)
            elseif step == 4 then
                throw(me, s, spec)
            elseif step == 8 then
                back(me, s)
            else
                error(string.format("AntiDmg's counter: phase %#x reads past off_80EE920", step))
            end
        end,
    }
end

return counter
