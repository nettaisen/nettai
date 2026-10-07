--!strict
-- EXE4's AirShot, the chips' action 0x23 (0x080ECB74): on its first tick the navi shows animation 12, opens its counter
-- window (0x0800BA66), is using an action and holds its air shooter (an attachment, its animation the navi's number),
-- with the shot's sound (0x8D, a spreader's too); on the 6th the shot from the panel ahead, 24 pixels up, with the
-- attack's damage word (its +6 halfword added); after 10 ticks the recovery, then the attack's exit (0x0800C9FC).
-- EXE4's own, not EXE5's and EXE6's (@exelib/airshot: animation 9, an overlay, 11 ticks of recovery). Its recovery
-- (0x080ECC02) sets its count to the attack's variant (+3: the store's 10 is overwritten before it), and to 30 when
-- that isn't 0: AirShot's variant 0 leaves on the recovery's first tick. Each chip composes its own: the shooter
-- (attachment row 9) and the shot (a projectile row: the chip's parameter +0x10, AirShot's 4).

local attachment = require("../objects/attachment")
local projectile = require("./projectile")

local airshot = {}

-- The air shooter (attachment row 9, 0x080CD524: 0c-18 in palette 0).
airshot.shooter = attachment.look { sprite = asset.sprite("air-shooter"), palette = 0 }

export type AirShotSpec = {
    id: string,
    shooter: AttachmentLook,
    shot: ProjectileVariant,
    -- A variant other than 0: 30 ticks of recovery (0x080ECC12); none else.
    long_recovery: boolean?,
}

type State = { timer: number }

-- The phases, on the attack's step (0x080ECB8C's table).
local FIRE, RECOVER = 0x0, 0x4
local ANIM = 12
-- The shot leaves on this tick of the count, and the phase ends on this one.
local SHOT_TICK, FIRE_TICKS = 5, 10
-- The recovery's count of a variant other than 0.
local LONG_RECOVERY = 30
-- The shot's height, 16.16.
local SHOT_Z = 0x18_0000
-- The shooter's animation: NaviStats +0x23, the navi's number (MegaMan's 0).
local NAVI_NUMBER = 0
-- The shot's sound (0x8D, 0x080ECBC6).
local SHOT_SOUND = asset.sound("spreader")

-- The chips' action 0x23 with a chip's shooter and shot.
function airshot.action(spec: AirShotSpec): Action
    -- 0x080ECB94.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM)
            me:open_counter_window()
            s.timer = 0
            me:set_status("using_action", true)
            attachment.attach(me, spec.shooter, "related", NAVI_NUMBER)
            battle.play_sound(SHOT_SOUND)
            me.step_init = 4
            return
        end
        if s.timer == SHOT_TICK then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            projectile.spawn(me, me.panel_x + me.facing, me.panel_y, { variant = spec.shot, damage = damage, z = SHOT_Z })
        end
        s.timer += 1
        if s.timer == FIRE_TICKS then
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- 0x080ECC02.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = if spec.long_recovery then LONG_RECOVERY else 0
            me.step_init = 4
        end
        s.timer -= 1
        if s.timer < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = { timer = "i16" },
        -- 0x080ECB74.
        update = function(me: Object, s: State)
            local step = me.step
            if step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("AirShot phase %#x reads past its phase table", step))
            end
        end,
    }
end

return airshot
