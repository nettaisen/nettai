--!strict
-- EXE4's flame burners (the chips' action 0x22, 0x080ECAAE: EXE4's own). The navi (animation 13, a counter window)
-- holds up the burner (attachment row 0x0C: 0c-1e, its animation three times the navi's number) for 33 ticks; 5 ticks
-- in the burner's animation goes one on, the navi's to 17, and flames (./flame) rise two panels ahead in the variant's
-- pattern (0x080ECB34: FlmLine's column, 0x080D00D8; FlmCros's cross, 0x080D00F8), each burning for the chip's 40 ticks
-- (its parameters' second byte); then 11 ticks more, and back to idle without the burner.

local attachment = require("../../objects/attachment")
local flame = require("./flame")

local flmline = {}

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local BURNER = attachment.look { sprite = asset.sprite("flame-line-burner"), palette = 0 }
-- The ticks each flame burns (the chips' parameters' second byte: 40 for all of them).
local BURNS = 0x28

local ANIM, FIRE_ANIM = 13, 17
local HOLD_TICKS, FIRE_AT, RECOVER_TICKS = 32, 27, 10

type State = { timer: number }

-- Where a chip's flames rise around (x, y), the panel two ahead, with `element` and the damage word.
export type Flames = (me: Object, x: number, y: number, element: number, damage: number) -> ()

-- 0x080D00D8 (variant 0, FlmLine's): the column's flames, level, up, down.
function flmline.column(me: Object, x: number, y: number, element: number, damage: number)
    flame.spawn(me, x, y, element, damage, BURNS)
    flame.spawn(me, x, y - 1, element, damage, BURNS)
    flame.spawn(me, x, y + 1, element, damage, BURNS)
end

-- 0x080D00F8 (variant 1, FlmCros's): a cross of five, level, up, down, the panel behind it and the one past it.
function flmline.cross(me: Object, x: number, y: number, element: number, damage: number)
    flmline.column(me, x, y, element, damage)
    flame.spawn(me, x - me.facing, y, element, damage, BURNS)
    flame.spawn(me, x + me.facing, y, element, damage, BURNS)
end

-- A chip's burner: its key and where its flames rise.
function flmline.action(id: string, flames: Flames): Action
    -- 0x080ECACC: the burner up; 5 ticks in, its animation one on, the navi's 17 and the flames out; 33 ticks.
    local function hold(me: Object, s: State)
        if me.step_init == 0 then
            me.anim = ANIM
            me:open_counter_window()
            s.timer = HOLD_TICKS
            me:set_status("using_action", true)
            attachment.attach(me, BURNER, "related", 3 * NAVI_NUMBER)
            me.step_init = 4
        end
        if s.timer == FIRE_AT then
            local burner = me.related1
            if burner then
                burner.anim += 1
            end
            me.anim = FIRE_ANIM
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            flames(me, me.panel_x + 2 * me.facing, me.panel_y, me.attack_element, damage)
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080ECB52: 11 ticks, then back to idle without the burner.
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
        id = id,
        state = { timer = "u16" },
        -- 0x080ECAAE.
        update = function(me: Object, s: State)
            if me.step == 0 then
                hold(me, s)
            elseif me.step == 4 then
                recover(me, s)
            else
                error(string.format("FlmLine's phase %#x reads past its table (0x080ECAC4)", me.step))
            end
        end,
    }
end

return flmline
