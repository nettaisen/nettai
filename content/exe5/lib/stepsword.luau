--!strict
-- EXE5's step before a charged sword (AIAttackVars+4, `me.charged`: a soul's charged chip), as its Katanas (action
-- 0x54, 0x080EF9B0) and MoonBlds (action 0x53, 0x080EF7B0) take it, EXE6's StepSwrd's: by 1, two panels ahead
-- (0x080125D4, EXE6's `sub_8015B00`); by 2, or in ShadowSoul (soul 8), behind the first enemy body it finds ahead
-- (0x08012538), turned round to face it. With nowhere to step it doesn't swing. It steps back once its last
-- swing's animation ends (0x080EFBE4, 0x080EF938).
--
-- The actions' phases are the same: 0 the step, 4 the swings' set-up, 0xC the step back, 0x10 the recovery.

local stepsword = {}

-- The phases this module moves to, on the attack's step.
local SET_UP, STEP_BACK, RECOVER = 0x4, 0xC, 0x10

-- The panel a step goes to: on the field, with no body on it.
local PANEL_EXISTS, BODIES = 0x10000, 0x0F800000
-- The enemy bodies the step behind looks for, by side (0x080125C0).
local ENEMY_BODIES = { [0] = 0x04000000, [1] = 0x08000000 }
-- The rows it looks in, from its own (0x080125C8).
local ROW_ORDER = { { 1, 2, 3 }, { 2, 1, 3 }, { 3, 1, 2 } }
-- The soul that steps behind the enemy.
local SHADOW_SOUL = 8

local AFTERIMAGE_SHADER = 0xFFE0

-- What the step keeps (+0x34): whether it turned round to step behind the enemy.
export type State = { turned: number }

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The sprite's flip: the side's, turned round or not.
function stepsword.flip(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- `sub_800F2C6` and 0x0801A51C: turn round (the sprite, and the HP number's place, which the engine doesn't draw).
local function turn(me: Object)
    me.flip = bit32.bxor(me.flip, 1)
    me.sprite:set_flip(stepsword.flip(me))
end

-- 0x08012538: behind the first enemy body ahead, column by column from its own, its own row first: the panel past
-- it, if that is on the field and empty. None once the columns leave the field.
local function behind_enemy(me: Object): (number?, number?)
    local x = me.panel_x
    local dir = me.facing
    local enemies = ENEMY_BODIES[me.alliance]
    while field.valid(x, me.panel_y) do
        for _, y in ROW_ORDER[me.panel_y] do
            if bit32.btest(field.flags(x, y), enemies) and field.check(x + dir, y, PANEL_EXISTS, BODIES) then
                return x + dir, y
            end
        end
        x += dir
    end
    return nil, nil
end

stepsword.behind_enemy = behind_enemy

-- The swords' slash's step (action 0x13, 0x080EBD04: @exelib/swords/slash with EXE5's `steps`): none uncharged; else,
-- once `set_off` lets it (not while moving or sliding), behind the enemy in ShadowSoul or by a charge of 2, two
-- panels ahead by 1, none by any other charge (set off all the same).
function stepsword.kind(me: Object, set_off: (Object) -> boolean): ("step" | "warp")?
    local mode = me.charged
    if mode == 0 or not set_off(me) then
        return nil
    end
    local soul = battle.navi(me.alliance).form.soul
    if (soul and soul.number == SHADOW_SOUL) or mode == 2 then
        return "warp"
    elseif mode == 1 then
        return "step"
    end
    return nil
end

-- 0x080125D4: two panels ahead, if on the field and empty.
local function two_ahead(me: Object): (number?, number?)
    local x, y = me.panel_x + 2 * me.facing, me.panel_y
    if field.check(x, y, PANEL_EXISTS, BODIES) then
        return x, y
    end
    return nil, nil
end

-- Phase 0: straight to the swings' set-up, or for a charged sword, the step. A MoonBld marks its holder moving
-- (`moving`: it clears that when it ends) once it may step.
function stepsword.start(me: Object, s: State, moving: boolean)
    set_step(me, SET_UP)
    local mode = me.charged
    if mode == 0 then
        return
    end
    -- sub_800E5FC: not while moving or sliding.
    if me:status("moving") or me:status("sliding") then
        return
    end
    if moving then
        me:set_status("moving", true)
    end
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    local soul = battle.navi(me.alliance).form.soul
    local x, y
    if (soul and soul.number == SHADOW_SOUL) or mode == 2 then
        s.turned = 1
        x, y = behind_enemy(me)
    elseif mode == 1 then
        s.turned = 0
        x, y = two_ahead(me)
    else
        return
    end
    if not x or not y then
        s.turned = 0
        me:set_animation(4)
        set_step(me, STEP_BACK)
        return
    end
    battle.afterimage(me, me.pos, {
        anim = 0,
        flip = stepsword.flip(me),
        color_shader = AFTERIMAGE_SHADER,
        lifetime = 0x14,
        shadow = "ground",
    })
    me.panel_x, me.panel_y = x :: number, y :: number
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    if s.turned == 1 then
        turn(me)
    end
end

-- A charged swing's afterimage of its holder, in animation `anim`, for 0x1E ticks.
function stepsword.afterimage(me: Object, anim: number)
    battle.afterimage(me, me.pos, {
        anim = anim,
        flip = stepsword.flip(me),
        color_shader = AFTERIMAGE_SHADER,
        lifetime = 0x1E,
        shadow = "ground",
    })
end

-- The phase after the last swing: the step back for a charged sword, else the recovery.
function stepsword.after_swings(me: Object)
    set_step(me, if me.charged ~= 0 then STEP_BACK else RECOVER)
end

-- Phase 0xC: back where it stepped from once its animation ends, then the recovery.
function stepsword.step_back(me: Object, s: State)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    if s.turned == 1 then
        turn(me)
    end
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:unreserve_panel(me.panel_x, me.panel_y)
    me:update_collision_panels()
    me:set_coordinates_from_panel()
    set_step(me, RECOVER)
end

return stepsword
