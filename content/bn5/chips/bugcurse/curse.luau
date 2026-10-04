--!strict
-- BugCurse's curse (BN5's attack object #0x80, 0x080D6534; BN5's own): on the enemy navi's panel, shown (sprite 0C-3F,
-- a sound), it waits 59 ticks, then over 5 ticks (3 in the battle flag 0x40 mode) hits the panel once a tick with
-- a bug each (0x080D6644; 0x080D664E in that mode): a one-tick hit (breaking, hitting navis, no spark, running while
-- dimmed) carrying the bug code and its argument. Then its controller (./controller), which waits on it, moves on.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

export type State = {
    -- The controller waiting on it (its flag, +0x4C: the controller's +0x0C).
    controller: Object?,
}

local curse = {}

local SPRITE = asset.sprite("bn5:bug-curse")
local SOUND = asset.sound("bn5:trap-vanish")
-- Its wait before the bugs.
local WAIT = 0x3A
-- The bugs it hits with, a tick each (the code in the low byte, its argument in the high), and in the battle flag 0x40
-- mode.
local BUGS: { number } = { 0x0124, 0x1418, 0x1419, 0x04FF, 0x04FE }
local BUGS_PER_PLAYER: { number } = { 0x1218, 0x04F9, 0x0814 }

-- Its controller is done waiting on it (the flag it gave it, cleared).
local function release(me: Object)
    local c = (me.state :: State).controller
    if c then
        (c.state :: any).waiting = false
    end
end

-- 0x080D6680: a one-tick hit on its panel carrying `bug` (code | argument << 8), running while dimmed.
local function hit(me: Object, bug: number)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.breaking,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 0,
        bug = bit32.band(bug, 0xFF),
        bug_arg = bit32.rshift(bug, 8),
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- Action 0 (0x080D65C8): 59 ticks (to -1).
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_action(4)
    end
end

-- Action 4 (0x080D65E8): a bug a tick, 5 of them (3 in the battle flag 0x40 mode); then the controller moves on, and
-- it goes.
local function bugs(me: Object)
    local per_player = battle.per_player_gauges()
    if me.phase_init == 0 then
        me.timer = if per_player then 3 else 5
        me.timer2 = 0
        me.phase_init = 4
    end
    local n = me.timer2
    if n < 5 then
        local list = if per_player then BUGS_PER_PLAYER else BUGS
        local bug = list[n + 1]
        if bug == nil then
            error(string.format("BugCurse's curse: bug %d reads past 0x080D664E", n))
        end
        hit(me, bug)
        me.timer2 = n + 1
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        release(me)
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = wait, [4] = bugs }

local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("BugCurse's curse: action %#x reads past its table (0x080D65C0)", me.action))
    end
    action(me)
end

-- 0x080D6558: its sprite (animation 0, a frame in, its first part drawn with it), shown, facing its way, on its
-- panel; a sound; its first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_animation(0)
    sprite:step()
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

curse.kind = define.kind {
    id = "bn5:bugcurse/curse",
    pool = "attack",
    state = { controller = "object" },
    -- 0x080D6534.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080D6654: a curse on (x, y) with `element` and the damage word, of `controller`'s side, which waits on it;
-- running while dimmed. (Its position is the spawner's registers: the row, the element and 0.)
function curse.spawn(controller: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(curse.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = controller.alliance
        local st = o.state :: State
        st.controller = controller
        o.run_while_dimmed = true
    end
    return o
end

return curse
