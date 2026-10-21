--!strict
-- C-SCOPE's scopes (the DS's effect object 0xA4, 0x020A1C9C): SearchMan's sight (10-23) on a panel of the other side's,
-- walking a diagonal from where it came: a step every six ticks (with a sound) through the offsets 0, 1, 2, 1 (each
-- that many columns forward and rows down, the second scope's forward the other way), four steps a round; after eight
-- rounds it rests 30 ticks, and walks on. Its actor stops it (it stays where it is) and ends it.

local scope = {}

type State = {
    -- Where it came (the DS's Param3, Param4) and its step (+0x7C).
    origin_x: number,
    origin_y: number,
    step: number,
    -- Stopped (its Param2, which 0x020A1E14 sets), and the rounds walked (its +0x0C).
    stopped: boolean,
    rounds: number,
}

local SPRITE = asset.sprite("search-scope")
local SOUND_STEP = asset.sound("erase-man")
-- Its offsets by step (0x02135BA8), a step's ticks, its rounds before it rests, the rest.
local OFFSETS = { 0, 1, 2, 1 }
local STEP_TICKS, ROUNDS, REST = 6, 8, 0x1E
local WALK, PAUSE = 0x0, 0x4

-- 0x020A1DD8: on its offset's panel, on the ground.
local function place(me: Object)
    local s = me.state :: State
    local d = OFFSETS[s.step + 1]
    me.panel_x = s.origin_x + d * me.facing
    me.panel_y = s.origin_y + d
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
end

-- Action 0 (0x020A1D54): unless stopped, a step every six ticks; its rounds counted, from eight on a rest after each.
local function walk(me: Object)
    local s = me.state :: State
    if s.stopped then
        return
    end
    local t = me.timer + 1
    me.timer = t
    if t < STEP_TICKS then
        return
    end
    battle.play_sound(SOUND_STEP)
    me.timer = 0
    local k = s.step + 1
    if k > #OFFSETS - 1 then
        local rounds = s.rounds + 1
        s.rounds = rounds
        if rounds >= ROUNDS then
            s.step = 0
            me:set_action(PAUSE)
            return
        end
        k = 0
    end
    s.step = k
    me.phase, me.phase_init = 0, 0
end

-- Action 4 (0x020A1D30): 30 ticks' rest, then it walks on.
local function pause(me: Object)
    if me.phase_init == 0 then
        me.timer = 0
        me.timer2 = REST
        me.phase_init = 4
    end
    local t = me.timer2 - 1
    me.timer2 = t
    if t <= 0 then
        me:set_action(WALK)
    end
end

local function tick(me: Object)
    local a = me.action
    if a == WALK then
        walk(me)
    elseif a == PAUSE then
        pause(me)
    else
        error(string.format("C-SCOPE's scope: action %#x reads past its table (0x02135BA0)", a))
    end
    place(me)
end

scope.kind = new.kind {
    id = "c-scope/scope",
    pool = "effect",
    state = { origin_x = "u8", origin_y = "u8", step = "u8", stopped = "bool", rounds = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x020A1CB0: its sprite, no shadow, shown; its future panel two ahead and two down (nothing reads it); no
            -- rounds yet.
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "hidden"
            me.anim, me.anim_loaded = 0, 0xFF
            me.visible = true
            me.future_panel_x, me.future_panel_y = me.panel_x + 2 * me.facing, me.panel_y + 2
            ;(me.state :: State).rounds = 0
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x020A1DA0: a scope for `actor` from (x, y), the second (`which` 1) walking the other way; it runs while dimmed.
function scope.spawn(actor: Object, x: number, y: number, which: number): Object?
    local o = battle.spawn(scope.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    local s = o.state :: State
    s.origin_x, s.origin_y = x, y
    o.alliance = actor.alliance
    o.flip = bit32.bxor(actor.flip, which)
    o.run_while_dimmed = true
    return o
end

-- 0x020A1E2C: it rests (its eight rounds walked).
function scope.paused(o: Object): boolean
    return o.action == PAUSE
end

-- 0x020A1E14: it stops where it is.
function scope.stop(o: Object?)
    if o then
        (o.state :: State).stopped = true
    end
end

-- 0x020A1E20: it ends.
function scope.finish(o: Object?)
    if o then
        o:set_lifecycle("destroy")
    end
end

return scope
