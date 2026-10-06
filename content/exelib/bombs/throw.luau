--!strict
-- The bombs' and seeds' throw, as EXE5 and EXE6 share it (EXE6's action 0x12,
-- `sub_80EB628`; EXE5's 0x12), of a game's `ThrowLook` (its sound and
-- attachment kind):
-- the navi winds up with the thing in hand (an attachment), throws it on
-- the tenth tick (it leaves from 4 pixels ahead, 48 up), and stands for 11
-- more, then 6 ticks of recovery. Each chip composes its own
-- (`throw.action { held = ..., thrower = ... }`): what it holds, and what
-- throws what it throws (docs/design/content-model-v2.md §5.1).

local attachment = require("../attachment")

local throw = {}

export type State = {
    -- Ticks into the throw, then the recovery's countdown.
    timer: number,
}

-- One state for every throw: a throw continues the last one's attack
-- variables, as the original's one action does.
throw.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local THROW, RECOVER = 0x0, 0x4

local ANIM_THROW = 6
-- The throw leaves on this tick, and the phase ends on this one.
local RELEASE_TICK, THROW_TICKS = 9, 0x15
local RECOVER_TICKS = 5
-- Where the thrown thing starts: ahead of the navi and up, 16.16.
local RELEASE_AHEAD, RELEASE_UP = 4 * 0x10000, 0x30 * 0x10000

local function set_phase(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- Where a thrown thing leaves from.
function throw.release_point(me: Object): Vec3
    local p = me.pos
    return Vec3.new(p.x + me.facing * RELEASE_AHEAD, p.y, p.z + RELEASE_UP)
end

-- The damage word a thrown thing carries: the attack's damage and hit
-- parameter plus the Atk+ bonus.
function throw.damage_word(me: Object): number
    return int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
end

-- A chip's throw, of a game's `look`.
function throw.action(spec: ThrowSpec, look: ThrowLook): Action
    local SOUND = look.sound
    -- `sub_80EB644`: wind up, throw on the tenth tick, stand.
    local function wind_up(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_THROW)
            me:open_counter_window()
            attachment.attach(look.attachment, me, spec.held, "related", { anim = spec.held_anim, palette_add = spec.held_palette })
            battle.play_sound(SOUND)
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if s.timer == RELEASE_TICK then
            spec.thrower.throw(me, throw.release_point(me), throw.damage_word(me))
            -- Let go of what it held.
            me.related1 = nil
            me.overlay = nil
        end
        local t = s.timer + 1
        s.timer = t
        if t >= THROW_TICKS then
            set_phase(me, RECOVER)
        end
    end

    -- `sub_80EB758`: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- `sub_80EB628`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == THROW then
            wind_up(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("throw phase %#x reads past off_80EB63C", step))
        end
    end

    return new.action { id = spec.id, state = throw.STATE, args = spec, update = update }
end

return throw
