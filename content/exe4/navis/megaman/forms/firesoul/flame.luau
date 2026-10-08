--!strict
-- FireSoul's flame (action 0x19, 0x080EC44A: EXE4's own), which its charged shot and its charged fire chips start,
-- each its own (`flame.action`) with its flames' count. The navi raises its claw (attachment row 0x2C, 0c-45, its
-- animation three times the navi's number, in the related slot) in animation 13, the counter window open, and sends a
-- flame (attack object 9: @exelib/elemrage/flame, its own kind) along its row from the panel ahead, 4 pixels back and
-- 26 up (0x080EC500 by the navi's number: MegaMan's), its time 30 ticks (the parameters' second byte); the roar sounds
-- every 16 ticks from the first; 30 ticks, then 6 more, and it is back to idle without the claw.

local attachment = require("../../../../objects/attachment")
local collision = require("../../../../rules/collision")
local flame_common = require("@exelib/elemrage/flame")
local sparks = require("../../../../lib/sparks")

local flame = {}

-- The flame (attack object 9, 0x080CE2D8): its sprite (0c-0e); FireSoul's leave no panel and send none beside.
local FLAME = flame_common.make_kind {
    id = "firesoul/flame",
    sprite = asset.sprite("rage-flame"),
    spreads = false,
    attack = collision.attack,
    attack_480 = collision.attack_480,
    hits = collision.hits_navis,
    spark = sparks.of_element,
}

-- The navi's number (NaviStats +0x23: MegaMan's 0) and its row of 0x080EC500: the flame's offset ahead and height.
local NAVI_NUMBER = 0
local OFFSET, HEIGHT = -4, 26
-- Its flames' time (the parameters' second byte, 0x1E).
local TICKS = 30

local CLAW = attachment.look { sprite = asset.sprite("rage-claw"), palette = 0 }
local ROAR = asset.sound("roar")
local ANIM = 13
local LAST_TICK, RECOVER_TICKS = 30, 5

type State = {
    -- Ticks into the action (AIAttackVars +0x10).
    timer: number,
}

-- FireSoul's flame of `id` whose flames run `count` panels (the parameters' first byte).
function flame.action(spec: { id: string, count: number }): Action
    -- 0x080EC468: on its first tick the flame out, the claw raised; the roar every 16 ticks; on after the 30th.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
            FLAME.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, HEIGHT * 0x10000, {
                count = spec.count,
                spread = false,
                ticks = TICKS,
                offset = OFFSET,
                paralyzes = false,
                attack_480 = false,
                while_dimmed = false,
                leaves = nil,
                palette = 0,
                hit = false,
                sent = false,
            }, damage)
            me.anim = ANIM
            me:open_counter_window()
            attachment.attach(me, CLAW, "related", 3 * NAVI_NUMBER)
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if bit32.band(s.timer, 15) == 0 then
            battle.play_sound(ROAR)
        end
        s.timer += 1
        if s.timer >= LAST_TICK then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080EC51C: 6 ticks, then back to idle without the claw.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = { timer = "u16" },
        -- 0x080EC44A.
        update = function(me: Object, s: State)
            if me.step == 0 then
                fire(me, s)
            elseif me.step == 4 then
                recover(me, s)
            else
                error(string.format("FireSoul's flame's phase %#x reads past 0x080EC460", me.step))
            end
        end,
    }
end

return flame
