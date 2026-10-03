--!strict
-- SearchMan, the navi of the SearchMan chips (BN5's actor object #0x31, 0x080C11D8; BN5's own, built on BN6's
-- Colonel template), brought by their navi chip controller. He fades in on his user's panel (and goes if it
-- isn't solid), raises his rifle (animation 16) and sends his scope (./scope) sweeping the enemy area until it
-- locks on; then five shots (./shot) on that panel, 10 ticks apart, the last powered if his user pressed B with
-- the lock; he lowers his rifle, and goes.

local navi_chips = require("@common/navi-chips/navi")
local scope = require("./scope")
local shot = require("./shot")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x64).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS): his palette, his scope's speed.
    level: number,
    -- The gray of his color shader, fading in (+0x19).
    gray: number,
    -- His scope's lock was powered (+5), his shots left (+6), this one's fired (+7).
    powered: boolean,
    shots: number,
    fired: boolean,
}

local searchman = {}

local SPRITE = asset.sprite("bn5:searchman")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_SHOT = asset.sound("bn5:col-army-2")

-- By the level (0x0800DA8A: 0x0801D737's navi 3 count by 0x0800DAE8's row): SP's 2, DS's 3.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 2, 3 }
local FADE_GRAY = 0x1F
local SOLID = 0x10

-- Action 0, phase 0 (0x080C1294): he fades in from gray 31, 2 a tick, flickering.
local function fade_in(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        s.gray = FADE_GRAY
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0
        me.phase_init = 4
        return
    end
    me.visible = true
    local t = int.u16(me.timer + 1)
    me.timer = t
    if bit32.band(t, 1) ~= 0 then
        me.visible = false
    end
    local gray = s.gray - 2
    s.gray = int.u8(gray)
    if gray > 0 then
        return
    end
    me.visible = true
    s.gray = 0
    me.phase, me.phase_init = 4, 0
end

-- Action 0, phase 4 (0x080C12E4): 20 ticks; then the scope, or he goes off a panel that isn't solid.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x14
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    if field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:set_action(4)
    else
        me:set_lifecycle("destroy")
    end
end

-- Action 4, phase 0 (0x080C133C): his rifle up (animation 16), his scope out; until it locks on.
local function aim(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 0x10
        s.powered = false
        me.future_panel_x, me.future_panel_y = 0, 0
        scope.spawn(me, me.related1 :: Object, s.level)
        me.phase_init = 4
        return
    end
    if me.future_panel_x ~= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- A shot's start (animation 17 from its first frame), 10 ticks.
local function load(me: Object, s: State)
    me.anim, me.anim_loaded = 0x11, 0x12
    me.timer = 0xA
    battle.play_sound(SOUND_SHOT)
    s.fired = false
    me.phase_init = 4
end

-- Action 4, phase 4 (0x080C136C): five shots on the locked panel, at 7 of each one's 10 ticks; the last powered.
local function fire(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        s.shots = 5
        load(me, s)
        return
    end
    if me.timer == 7 and not s.fired then
        s.fired = true
        shot.spawn(me, me.future_panel_x, me.future_panel_y, s.powered and s.shots == 1)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    s.shots -= 1
    if s.shots > 0 then
        load(me, s)
        return
    end
    me.phase, me.phase_init = 8, 0
end

-- Action 4, phase 8 (0x080C13CE): his rifle down; 30 ticks.
local function lower(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080C13F6): he goes (animation 4 from its first frame); 5 ticks, then out of sight.
local function leave(me: Object)
    if me.phase == 0 then
        me.anim, me.anim_loaded = 4, 0
        me.timer = 5
        me.phase = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.visible = false
        me:set_lifecycle("destroy")
    end
end

local ACTIONS: { [number]: { [number]: (Object) -> () } } = {
    [0] = { [0] = fade_in, [4] = ready },
    [4] = { [0] = aim, [4] = fire, [8] = lower },
}

-- 0x080C1250: the action, the sprite, his color shader from the gray (0x080C1446).
local function tick(me: Object)
    if me.action == 8 then
        leave(me)
    else
        local phases = ACTIONS[me.action]
        if not phases then
            error(string.format("SearchMan action %#x reads past his table (0x080C1268)", me.action))
        end
        local f = phases[me.phase]
        if not f then
            error(string.format("SearchMan action %#x phase %#x reads past its table", me.action, me.phase))
        end
        f(me)
    end
    me:update_sprite()
    me.sprite.color_shader = (me.state :: State).gray * 0x421
end

-- 0x080C11F8: on his panel with his shadow, his palette by the level, faded out (gray 0x7FFF shader).
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = PALETTES[s.level] or 0
    sprite.color_shader = 0x7FFF
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

searchman.kind = define.kind {
    id = "bn5:serchman/navi",
    pool = "actor",
    state = {
        controller = "object",
        level = "u8",
        gray = "u8",
        powered = "bool",
        shots = "u8",
        fired = "bool",
    },
    -- 0x080C11D8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080C1422: the controller moves on, and he is freed.
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
        end
    end,
}

-- 0x080C1458: SearchMan for `user`, reporting to `controller` (the summon table's entry 2), of a chip of `level`.
function searchman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(searchman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- SearchMan's, SerchMnSP's and SerchMnDS's (their Param1s 0, 3 and 4).
searchman.summon = searchman.summon_at(0)
searchman.summon_sp = searchman.summon_at(3)
searchman.summon_dark = searchman.summon_at(4)

return searchman
