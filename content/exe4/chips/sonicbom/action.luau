--!strict
-- SonicBom's and SprSonic's action (EXE4's action 0x37, 0x080ED7E6: EXE4's own; EXE5's action 0x2B grew from it):
-- the navi swings its sword (animation 5, a counter window, the sword's swing, the blade in its related slot) and on
-- the swing's tenth tick throws a sonic boom (../../objects/sonic_boom, attack 0x58) from the panel ahead, of the
-- chip's hit modifier and going through by its second parameter (the record's +0x10 and +0x11: SonicBom's 3 and no,
-- SprSonic's 1 and yes). 22 ticks, then 6, and back to idle without the blade. A charged one (AIAttackVars +4) first
-- steps two panels ahead (on the field, nobody on it; not while it can't move: 0x0800AD2A), throws a boom of hit
-- modifier 1 and steps back once its swing's animation ends; with nowhere to step it doesn't swing (animation 4, its
-- load not forced, then the step back). EXE5's but: no afterimages, no step behind the enemy, no arm to drop. The
-- blade's animation is ProtoSoul's 13 or the navi's number (lib/swords' `blade_anim`). Each chip makes its own
-- (`action.make`).

local attachment = require("../../objects/attachment")
local sonic_boom = require("@exelib/swords/sonic_boom")
local sonic_boom_kind = require("../../objects/sonic_boom").kind
local swords = require("../../lib/swords")

local action = {}

type State = {
    -- Ticks left in the phase (AIAttackVars +0x10). (The swings left are the navi's `attack_count`.)
    timer: number,
}

local START, SET_UP, SWING, STEP_BACK, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

local ANIM_SWING, ANIM_NO_STEP = 5, 4
local SWING_TICKS, THROW_AT, RECOVER_TICKS = 0x15, 0x0C, 5
-- A charged one's boom's hit modifier (its first parameter becomes 1).
local CHARGED_HIT_MOD = 1
-- The panel the step goes to: on the field (every panel type's flag 0x10000), with no body on it.
local PANEL_EXISTS, BODIES = 0x10000, 0x0F800000

local function set_step(me: Object, step: number)
    me.step, me.step_init = step, 0
end

-- 0x080ED810: on to the set-up; a charged one steps two panels ahead first.
local function start(me: Object)
    set_step(me, SET_UP)
    if me.charged == 0 or not me:can_move() then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    local x, y = me.panel_x + 2 * me.facing, me.panel_y
    if field.check(x, y, PANEL_EXISTS, BODIES) then
        me.panel_x, me.panel_y = x, y
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        return
    end
    me.anim = ANIM_NO_STEP
    set_step(me, STEP_BACK)
end

-- 0x080ED87E: the swing; on the timer's 12 the boom.
local function swing(me: Object, s: State, hit_mod: number, pierces: boolean)
    if me.step_init == 0 then
        me:set_animation(ANIM_SWING)
        me:open_counter_window()
        battle.play_sound(swords.swing)
        attachment.attach(me, swords.blade, "related", swords.blade_anim(me))
        me:set_status("using_action", true)
        s.timer = SWING_TICKS
        me.step_init = 4
    end
    if s.timer == THROW_AT then
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        sonic_boom.throw(sonic_boom_kind, me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage, {
            hit_mod = if me.charged ~= 0 then CHARGED_HIT_MOD else hit_mod,
            pierces = pierces,
        })
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t >= 0 then
        return
    end
    local left = me.attack_count - 1
    me.attack_count = left
    if left > 0 then
        me.related1 = nil
        me.step_init = 0
        return
    end
    set_step(me, if me.charged ~= 0 then STEP_BACK else RECOVER)
end

-- 0x080ED92A: back where it stepped from once its animation ends.
local function step_back(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:unreserve_panel(me.panel_x, me.panel_y)
    me:update_collision_panels()
    me:set_coordinates_from_panel()
    set_step(me, RECOVER)
end

-- 0x080ED950: 6 ticks, then back to idle without the blade.
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

-- The action of a chip whose parameters are `hit_mod` (its boom's hit modifier) and `pierces` (it goes through).
function action.make(id: string, hit_mod: number, pierces: boolean): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        -- 0x080ED7E6.
        update = function(me: Object, s: State)
            local step = me.step
            if step == START then
                start(me)
            elseif step == SET_UP then
                -- 0x080ED86C: one swing.
                me.attack_count = 1
                set_step(me, SWING)
            elseif step == SWING then
                swing(me, s, hit_mod, pierces)
            elseif step == STEP_BACK then
                step_back(me)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("SonicBom's phase %#x reads past its table (0x080ED7FC)", step))
            end
        end,
    }
end

return action
