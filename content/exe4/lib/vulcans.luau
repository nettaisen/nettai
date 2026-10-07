--!strict
-- EXE4's vulcans, the chips' action 0x1F (0x080EC842): the navi raises its gun, a tick later fires its variant's
-- shots, one every 11 ticks, each a bullet (objects/bullet) from the panel ahead at a height of 8, 16, 24 or 32 pixels
-- (an RNG2 draw, 0x080EC939), with the shot's sound; then lowers it for 11 ticks. Each chip composes its own from its
-- variant's rows: the gun (attachment row 13, DrkVulcn's 49), the shots (0x080EC934: 3, 5, 7, 12, 24) and the
-- bullet's variant (its parameter +0x10).

local attachment = require("../objects/attachment")
local bullet = require("../objects/bullet")

local vulcans = {}

local GUN = asset.sprite("vulcan")
-- The guns (attachment rows 13 and 49: 0c-1d in palettes 0 and 3).
vulcans.gun = attachment.look { sprite = GUN, palette = 0 }
vulcans.dark_gun = attachment.look { sprite = GUN, palette = 3 }

export type VulcanSpec = {
    id: string,
    gun: AttachmentLook,
    shots: number,
    shot: BulletVariant,
}

type State = { timer: number, shots: number }

-- The phases, on the attack's step (0x080EC858's table).
local RAISE, FIRE, LOWER = 0x0, 0x4, 0x8
-- The gun's animation: the navi's number (NaviStats +0x23, MegaMan's 0) three times.
local NAVI_NUMBER = 0
-- The shots' heights, in pixels, by an RNG2 draw's low two bits (0x080EC939).
local HEIGHTS: { number } = { 8, 16, 24, 32 }
-- Ticks between shots, less one.
local SHOT_WAIT = 10
local SHOT_SOUND = asset.sound("vulcan")

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The gun's next frame of animation.
local function step_gun(me: Object)
    local gun = me.related1
    if gun then
        gun.anim += 1
    end
end

-- The chips' action 0x1F with a variant's rows.
function vulcans.action(spec: VulcanSpec): Action
    -- 0x080EC864: raise the gun, for a tick.
    local function raise(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(13)
            me:open_counter_window()
            attachment.attach(me, spec.gun, "related", 3 * NAVI_NUMBER)
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 1
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            set_step(me, FIRE)
        end
    end

    -- 0x080EC8BA: the shots, the first at once.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            s.shots = spec.shots
            step_gun(me)
            me:set_animation(17)
            s.timer = 0
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        s.timer = SHOT_WAIT
        local z = HEIGHTS[bit32.band(battle.rng_positive(), 3) + 1] * 0x10000
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        bullet.fire(me, me.panel_x + me.facing, me.panel_y, z, spec.shot, me.attack_element, damage)
        battle.play_sound(SHOT_SOUND)
        s.shots -= 1
        if s.shots <= 0 then
            set_step(me, LOWER)
        end
    end

    -- 0x080EC940: lower the gun for 11 ticks, then back to idle.
    local function lower(me: Object, s: State)
        if me.step_init == 0 then
            step_gun(me)
            me:set_animation(13)
            s.timer = SHOT_WAIT
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    -- 0x080EC842.
    local function update(me: Object, s: State)
        local step = me.step
        if step == RAISE then
            raise(me, s)
        elseif step == FIRE then
            fire(me, s)
        elseif step == LOWER then
            lower(me, s)
        else
            error(string.format("vulcan phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = { timer = "i16", shots = "i16" }, update = update }
end

return vulcans
