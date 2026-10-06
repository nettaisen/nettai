--!strict
-- AuraHed's action (the original's 0x43, `sub_80EE634`): the navi raises
-- its buster and fires an aura head (attack object #0x92, ./head) along
-- its row, recoiling half a panel and back over 31 ticks. A stream
-- (StreamHd, the original's variant 1) fires five in a row, 16 ticks
-- apart, counting their palettes up from its head's (the original counts
-- in the attack's first parameter), the last one stronger and with the
-- longer recoil.

local buster = require("../../lib/buster")
local head = require("./head")

local action = {}

export type State = {
    -- Ticks left in the recoil (AIAttackVars+0x10).
    timer: number,
    -- A stream's heads so far: the next one's palette.
    heads: number,
}

-- One state for every AuraHed: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16", heads = "u8" }

-- The phases, on the attack's step.
local RAISE, FIRE, RECOIL = 0x0, 0x4, 0x8

local ANIM_RAISE = 0x0E
-- A stream fires until its count reaches this.
local LAST = 4
-- The recoil's start speed and slowdown, 16.16, and its ticks between a
-- stream's heads and after the last.
local RECOIL_SPEED, RECOIL_DRAG = 0x3_0000, 0x8000
local STREAM_TICKS, RECOIL_TICKS = 0x0F, 0x1E

-- A chip's AuraHed: its head, and whether it streams.
function action.action(spec: AuraHeadSpec): Action
    local stream = spec.stream == true

    -- More heads to come (a stream, not yet at its last).
    local function streaming(s: State): boolean
        return stream and s.heads ~= LAST
    end

    -- `sub_80EE654`: raise the buster.
    local function raise(me: Object, s: State)
        me:set_animation(ANIM_RAISE)
        buster.raise_arm(me)
        me:set_status("using_action", true)
        s.heads = spec.head.palette
        me.step = FIRE
        me.step_init = 0
    end

    -- `sub_80EE66E`: fire a head from the navi's panel, and start the
    -- recoil.
    local function fire(me: Object, s: State)
        local what = spec.head
        if stream then
            -- A stream's head: its count as its palette, and the last one
            -- stronger.
            what = {
                palette = s.heads,
                speed = what.speed,
                strong = what.strong or s.heads == LAST,
                far = what.far,
            }
        end
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        head.spawn(me, me.panel_x, me.panel_y, me.attack_element, what, damage)
        s.timer = if streaming(s) then STREAM_TICKS else RECOIL_TICKS
        me.vel = Vec3.new(RECOIL_SPEED, me.vel.y, me.vel.z)
        me.step = RECOIL
        me.step_init = 0
    end

    -- `sub_80EE6A4`: slide back, slowing; then (a stream) the next head, or
    -- back to the panel and to idle.
    local function recoil(me: Object, s: State)
        local v = me.vel.x
        if v ~= 0 then
            local p = me.pos
            me.pos = Vec3.new(p.x - me.facing * v, p.y, p.z)
            me.vel = Vec3.new(v - RECOIL_DRAG, me.vel.y, me.vel.z)
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        me:set_coordinates_from_panel()
        if streaming(s) then
            s.heads += 1
            me.step = FIRE
            me.step_init = 0
            return
        end
        me.overlay = nil
        me:exit_attack()
    end

    -- `sub_80EE634`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == RAISE then
            raise(me, s)
        elseif step == FIRE then
            fire(me, s)
        elseif step == RECOIL then
            recoil(me, s)
        else
            error(string.format("AuraHed phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
