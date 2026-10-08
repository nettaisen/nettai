--!strict
-- EXE4's ring throw (action 0x47, 0x080EDC5A: EXE4's own), each user its own (`rings.action`): with its parameter 3
-- (SandRing's) the navi throws in animation 0x12 holding nothing; with any other (the zap ring's, weapon routine 0x04)
-- in animation 13, holding the ring (attachment row 0x1D: 0c-36, by the navi's number), whose animation goes one on as
-- the ring flies. A counter window; 6 ticks on the ring (../objects/ring) flies from the panel ahead, 26 pixels up,
-- with the attack's element and damage word; 11 ticks, then 11 more, and back to idle (animation 0, its load not
-- forced).

local attachment = require("../objects/attachment")
local ring = require("../objects/ring")

local rings = {}

local TICKS, THROW_AT = 0x0A, 5
local HEIGHT = 0x1A_0000
local IDLE_ANIM = 0
-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local THROW, RECOVER = 0x0, 0x4

type State = {
    -- Ticks left in the phase (AIAttackVars +0x10).
    timer: number,
}

-- What a ring throw is: its key, the navi's animation, the ring it holds (none: nothing held), the ring it throws.
export type Spec = { id: string, anim: number, held: AttachmentLook?, ring: RingVariant }

function rings.action(spec: Spec): Action
    -- 0x080EDC78: the throw; on the timer's 5 the ring.
    local function throw(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me:set_animation(spec.anim)
            me:open_counter_window()
            local held = spec.held
            if held then
                attachment.attach(me, held, "related", NAVI_NUMBER)
            end
            s.timer = TICKS
            me.step_init = 4
            return
        end
        if s.timer == THROW_AT then
            local arm = me.related1
            if spec.held and arm then
                arm.anim = 1
            end
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            ring.spawn(me, spec.ring, me.panel_x + me.facing, me.panel_y, HEIGHT, me.attack_element, damage)
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- 0x080EDD00: 11 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me:set_status("using_action", false)
            me.anim = IDLE_ANIM
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = { timer = "u16" },
        -- 0x080EDC5A.
        update = function(me: Object, s: State)
            local step = me.step
            if step == THROW then
                throw(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("the ring throw's phase %#x reads past its table (0x080EDC6C)", step))
            end
        end,
    }
end

return rings
