--!strict
-- ShadeMan, the navi of the ShadeMan chips (BN5's actor object #0x19, 0x080BD0E8; BN5's own), brought by their
-- navi chip controller. He appears (animation 3), and for 31 ticks his user may put in a command; then he calls
-- out (animation 5), and 21 ticks later sends his crush (./crush) from the panel ahead; 61 ticks, a bow
-- (animation 4), and he goes.
--
-- His commands, read from his user's new presses while the battle is dimmed (a key of each step, its steps within
-- 10 ticks of each other, 0x080BD2B6): down, down or right, right or A paralyzes (without a flinch); down, down
-- or left, left or A confuses.

local navi_chips = require("@exelib/navi-chips/navi")
local crush = require("./crush")

-- What a command gives his crush (./crush's `Effect`).
type Effect = "none" | "paralyze" | "confuse"

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x78).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS), which picks his palette.
    level: number,
    -- The commands' progress (+0x60, +0x68: the step and the ticks left for the next) and what was put in
    -- (+0x70).
    steps: { number },
    ticks: { number },
    effect: Effect,
}

local shademan = {}

local SPRITE = asset.sprite("shademan")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_CALL = asset.sound("shademan-call")
local SOUND_COMMAND = asset.sound("ok-8b")

-- The commands (0x080BD194, 0x080BD1A0): a step is any of its keys.
local COMMANDS: { { effect: Effect, steps: { { Key } } } } = {
    { effect = "paralyze", steps = { { "down" }, { "down", "right" }, { "right", "a" } } },
    { effect = "confuse", steps = { { "down" }, { "down", "left" }, { "left", "a" } } },
}
local STEP_TICKS = 0xA

-- 0x080BD2B6: one command's next step: true once it is in.
local function command_step(user: Object, s: State, i: number): boolean
    local steps = COMMANDS[i].steps
    local n = s.steps[i]
    local any = false
    for _, key in steps[n + 1] do
        if user:dimmed_pressed(key) then
            any = true
        end
    end
    if any then
        s.ticks[i] = STEP_TICKS
        s.steps[i] = n + 1
        if steps[n + 2] == nil then
            battle.play_sound(SOUND_COMMAND)
            return true
        end
        return false
    end
    if n ~= 0 then
        local left = s.ticks[i] - 1
        if left ~= 0 then
            s.ticks[i] = int.u8(left)
        else
            s.steps[i], s.ticks[i] = 0, 0
        end
    end
    return false
end

-- Count the timer down; true once it went below 0.
local function past_zero(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

-- Action 0 (0x080BD1D4): the commands, until one is in; 31 ticks, then he calls out.
local function listen(me: Object)
    local s = me.state :: State
    if s.effect == "none" then
        local user = me.related1 :: Object
        if command_step(user, s, 1) then
            s.effect = COMMANDS[1].effect
        elseif command_step(user, s, 2) then
            s.effect = COMMANDS[2].effect
        end
    end
    if past_zero(me) then
        me.anim = 5
        battle.play_sound(SOUND_CALL)
        me.timer = 0x14
        me:set_action(4)
    end
end

-- Action 4 (0x080BD22C): 21 ticks; then his crush from the panel ahead; 61 ticks.
local function call(me: Object)
    if past_zero(me) then
        crush.spawn(me, me.panel_x + me.facing, me.panel_y, (me.state :: State).effect)
        me.timer = 0x3C
        me:set_action(8)
    end
end

-- Action 8 (0x080BD25C): then a bow (animation 4); 6 ticks.
local function bow(me: Object)
    if past_zero(me) then
        me.anim = 4
        me.timer = 5
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BD278): then he goes.
local function leave(me: Object)
    if past_zero(me) then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = listen, [4] = call, [8] = bow, [0xC] = leave }

local function act(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("ShadeMan action %#x reads past his table (0x080BD1C4)", me.action))
    end
    action(me)
end

-- 0x080BD11A: appearing (animation 3) on his panel with his shadow, his palette by the level; 30 ticks to put in a
-- command.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 3, 3
    sprite:set_animation(3)
    sprite.palette = if s.level == 4 then 6 else 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    battle.play_sound(SOUND_APPEAR)
    me.timer = 0x1E
    for i = 1, 2 do
        s.steps[i], s.ticks[i] = 0, 0
    end
    s.effect = "none"
    me:set_lifecycle("update")
    act(me)
end

shademan.kind = define.kind {
    id = "shademan/navi",
    pool = "actor",
    state = {
        controller = "object",
        level = "u8",
        steps = "u8[2]",
        ticks = "u8[2]",
        effect = { "none", "paralyze", "confuse" },
    },
    -- 0x080BD0E8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            -- 0x080BD10C: the controller moves on, and he is freed.
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BD288: ShadeMan for `user`, reporting to `controller` (the summon table's entry 1), of a chip of `level`.
function shademan.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(shademan.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- ShadeMan's, ShadeMnSP's and ShadeMnDS's (their Param1s 0, 3 and 4).
shademan.summon = shademan.summon_at(0)
shademan.summon_sp = shademan.summon_at(3)
shademan.summon_dark = shademan.summon_at(4)

return shademan
