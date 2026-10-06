--!strict
-- Meddy's throw (her own action 7, 0x080F1424), which her charged attack and her own chip both start: she swings
-- (animation 6) for 8 ticks, then throws a capsule (chips/meddy/capsule, attack object #0x0C) at the panel three
-- ahead, with the throw's sound; 12 ticks, then 15 standing, and back to idle. No counter window opens. The
-- capsule's parameters are the attack's (AIAttackVars +0x0C to +0x0E: the weapon routine's, or the chip's record's):
-- its variant, its line and its third; each use is an action of its own here, made with them.

local capsule = require("../../chips/meddy/capsule")

local throw = {}

-- The capsule's parameters: its variant (its palette, hit modifier and status), its line, and whether it runs while
-- dimmed (its third, 2).
export type Spec = { variant: CapsuleVariant, line: CapsuleLine, while_dimmed: boolean }

export type State = {
    -- AIAttackVars +0x10: the step's ticks left.
    timer: number,
}

-- The steps, on the attack's step.
local SWING, THROW, STAND = 0x0, 0x4, 0x8
local SWING_ANIM = 6
local SWING_TICKS, THROW_TICKS, STAND_TICKS = 8, 12, 15
local REACH = 3
local SOUND = asset.sound("minibomb-throw")

-- Count the step's ticks down; true when they run out.
local function count(s: State): boolean
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    return t <= 0
end

-- Her throw, its capsule of `spec` (`id`: the key compat names it by, if it does).
function throw.action(spec: Spec, id: string?): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        -- 0x080F1424
        update = function(me: Object, s: State)
            local step = me.step
            if step == SWING then
                -- 0x080F1444
                if me.step_init == 0 then
                    me.step_init = 4
                    me.anim = SWING_ANIM
                    s.timer = SWING_TICKS
                elseif count(s) then
                    me.step, me.step_init = THROW, 0
                end
            elseif step == THROW then
                -- 0x080F1468: the capsule's damage word is the attack's and its bonus.
                if me.step_init == 0 then
                    me.step_init = 4
                    capsule.throw(me, me.attack_element, {
                        variant = spec.variant,
                        line = spec.line,
                        target_x = me.panel_x + REACH * me.facing,
                        target_y = me.panel_y,
                        damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra),
                        while_dimmed = spec.while_dimmed,
                    })
                    battle.play_sound(SOUND)
                    s.timer = THROW_TICKS
                elseif count(s) then
                    me.step, me.step_init = STAND, 0
                end
            elseif step == STAND then
                -- 0x080F14AA
                if me.step_init == 0 then
                    me.step_init = 4
                    me.anim = 0
                    s.timer = STAND_TICKS
                elseif count(s) then
                    me:exit_attack()
                end
            else
                error(string.format("Meddy's throw's step %#x reads past its table (0x080F1438)", step))
            end
        end,
    }
end

return throw
