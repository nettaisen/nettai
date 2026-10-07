--!strict
-- EXE4's spreaders, the chips' action 0x1E (0x080EC760): the navi raises its gun and, once its count passes the
-- variant's delay, fires a bullet (objects/bullet) from the panel ahead at the variant's height, which bursts where
-- it finds something; 21 ticks of recovery. EXE6's spreader (@exelib/spreadr) in its shape, with EXE4's animation
-- (14), attachment and delays. Each chip composes its own from its variant's rows: the gun (0x080EC814, an
-- attachment row: Spreader's 0x0A, HeatShot's 6, Bubbler's 0x0E, DrkSpred's 0x32), the delay (0x080EC818: 1, 10, 10,
-- 1), the shot's height (0x080EC81C: 0x10, 0x10, 0, 0x10) and the bullet's variant (its parameter +0x10).

local attachment = require("../objects/attachment")
local bullet = require("../objects/bullet")

local spreaders = {}

local GUN = asset.sprite("spreader")
-- The guns (attachment rows 0x0A and 0x32: 0c-19 in palettes 0 and 3).
spreaders.gun = attachment.look { sprite = GUN, palette = 0 }
spreaders.dark_gun = attachment.look { sprite = GUN, palette = 3 }

export type SpreaderSpec = {
    id: string,
    gun: AttachmentLook,
    -- The count past which it fires.
    delay: number,
    -- The shot's height, in pixels.
    z: number,
    shot: BulletVariant,
}

type State = { timer: number }

-- The phases, on the attack's step.
local FIRE, RECOVER = 0x0, 0x4
-- The gun's animation: the navi's number (NaviStats +0x23, MegaMan's 0) twice.
local NAVI_NUMBER = 0
local SHOT_SOUND = asset.sound("spreader")

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The chips' action 0x1E with a variant's rows.
function spreaders.action(spec: SpreaderSpec): Action
    -- 0x080EC780: raise the gun; once the count passes the delay, the bullet.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(14)
            me:open_counter_window()
            attachment.attach(me, spec.gun, "related", 2 * NAVI_NUMBER)
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0
        end
        local t = s.timer + 1
        s.timer = t
        if t <= spec.delay then
            return
        end
        local gun = me.related1
        if gun then
            gun.anim += 1
        end
        battle.play_sound(SHOT_SOUND)
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        bullet.fire(me, me.panel_x + me.facing, me.panel_y, spec.z * 0x10000, spec.shot, me.attack_element, damage)
        set_step(me, RECOVER)
    end

    -- 0x080EC820: 21 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 20
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    -- 0x080EC760.
    local function update(me: Object, s: State)
        local step = me.step
        if step == FIRE then
            fire(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("spreader phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = { timer = "u16" }, update = update }
end

return spreaders
