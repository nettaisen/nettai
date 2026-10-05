--!strict
-- CosmoMan, the navi of the CosmoMan chips (EXE5's actor object #0x1D, 0x080BD9AC; EXE5's own, built on EXE6's
-- BlastMan template), brought by their navi chip controller. He appears (and goes at once off a solid panel),
-- takes the field objects around him, sends himself to the sky over the enemy area (./caller), where he calls his
-- meteors down, waits three seconds, and goes; the controller moves on.

local navi_chips = require("@exelib/navi-chips/navi")
local caller = require("./caller")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS), which picks his palette.
    level: number,
    -- The field objects he took (+0x60, their looks), and how many (+5).
    taken: { Identity? },
    count: number,
}

local cosmoman = {}

local SPRITE = asset.sprite("cosmoman")
local SOUND_APPEAR = asset.sound("appear")

-- By the level (0x080BDA20): the dark palette for DS's 4.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 0, 0xB }
-- Where his caller goes, by his side (0x080BDB24): the enemy area's middle panel.
local CALLER_PANELS: { [number]: { number } } = { [0] = { 5, 2 }, { 2, 2 } }
local SOLID = 0x10

-- 0x080BD9D0: on his panel, on the ground, his palette by the level, flipped by his side alone.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    local s = me.state :: State
    me.sprite.palette = PALETTES[s.level] or 0
    me.sprite:set_flip(me.alliance)
    me:set_lifecycle("update")
end

-- Enter the phase with a timer (a word store clears Timer2 too), or count down; true once it has run out.
local function countdown(me: Object, ticks: number, word: boolean): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- He is done: the controller moves on, and he goes.
local function done(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080BDA64): he appears; 10 ticks; then he stays, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 0xA, true) then
        me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0x10)
    end
end

-- 0x080BDB9C: the field objects around his panel (one panel or less each way) that can be taken, in the
-- registry's slot order: their looks kept, and the chip's removal asked of each. How many.
local function take(me: Object, s: State): number
    local n = 0
    for _, o in field.all_objects() do
        if math.abs(me.panel_x - o.panel_x) <= 1 and math.abs(me.panel_y - o.panel_y) <= 1 then
            local look = obstacle.junk_look(o)
            if look then
                s.taken[n + 1] = look
                n += 1
                obstacle.remove(o)
            end
        end
    end
    return n
end

-- Action 4 (0x080BDAB4): he takes what is around him; 60 ticks.
local function gather(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        local s = me.state :: State
        s.count = take(me, s)
    end
    if countdown(me, 0x3C, true) then
        me:set_action(8)
    end
end

-- Action 8 (0x080BDAE2): to the sky over the enemy area; 60 ticks.
local function launch(me: Object)
    if me.phase_init == 0 then
        local s = me.state :: State
        local at = CALLER_PANELS[me.alliance]
        caller.spawn(me, at[1], at[2], s.level, s.taken, s.count)
    end
    if countdown(me, 0x3C, true) then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BDB28): 180 ticks while his meteors fall.
local function wait(me: Object)
    if countdown(me, 0xB4, false) then
        me:set_action(0x10)
    end
end

-- Action 0x10 (0x080BDB4C): he goes (animation 4); 3 ticks.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 3, true) then
        done(me)
    end
end

local ACTIONS = { [0] = appear, [4] = gather, [8] = launch, [0xC] = wait, [0x10] = leave }

cosmoman.kind = define.kind {
    id = "cosmoman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8", taken = "identity[8]", count = "u8" },
    -- 0x080BD9AC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            -- 0x080BDA28: the battle's end ends him (the controller moves on).
            if battle.over() then
                done(me)
            else
                local action = ACTIONS[me.action]
                if not action then
                    error(string.format("CosmoMan action %#x reads past his table (0x080BDA50)", me.action))
                end
                action(me)
            end
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BDB76: CosmoMan for `user`, reporting to `controller` (the summon table's entry 16), of a chip of `level`.
function cosmoman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(cosmoman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- CosmoMan's and CosmoMnSP's (levels 0); CosmoMnDS's (4).
cosmoman.summon = cosmoman.summon_at(0)
cosmoman.summon_dark = cosmoman.summon_at(4)

return cosmoman
