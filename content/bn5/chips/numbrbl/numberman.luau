--!strict
-- NumbrBl's NumberMan (BN5's actor object #0x45, 0x080C2018; BN5's own: BN6's #0x45 draws the user): the navi himself,
-- on the panel the user warped out of. He appears (a sound) and, on a solid panel, stands 21 ticks; then sets balls
-- (@exelib/numbrbl/ball: the chip's first parameter and three) down in front of him, 24 ticks apart, waits 31 ticks
-- and leaves; off a solid panel he leaves at once. Leaving, he lets his controller go on. His sprite steps while
-- dimmed.

local common_ball = require("@exelib/numbrbl/ball")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")

local numberman = {}

export type State = {
    -- The controller waiting for him (+0x54: its `busy`).
    controller: Object?,
    -- His level (Param0): his palette's row, and the balls less three.
    level: number,
}

local SPRITE = asset.sprite("numberman")
local SOUND_APPEAR = asset.sound("appear")
-- His palette: navi 9's count (0x0801D737: 1) by the level's row (0x0800DAE8), the level his first parameter.
local PALETTE_ROWS = { [0] = 0, 0, 0, 2, 3, 0 }
local ANIM_APPEAR, ANIM_STAND, ANIM_LEAVE, ANIM_THROW = 3, 0, 4, 0x10
-- The balls sit 16 pixels up and wait 10 ticks (plus one) before rolling.
local BALL_Z, BALL_WAIT = 0x10_0000, 0xA

-- The balls (BN5's attack object #0x91, 0x080D895C).
local ball = common_ball.make {
    id = "numbrbl/ball",
    sprite = asset.sprite("number-ball"),
    sit_ticks = 0x1D,
    -- (0x112: 0x080D8A40 loads it as 19 + 255.)
    set_sound = asset.sound("place"),
    hit_sound = asset.sound("damage"),
    explosion = effects.explosion,
    is = collision.attack,
    hits = collision.hits_navis,
    -- 0x080D8ABA: parts 2 to 21.
    hidden_parts = 0x3FFF_FC00,
}
numberman.ball = ball

local function next_action(me: Object, a: number)
    me.action, me.phase, me.phase_init = a, 0, 0
end

-- Action 0 (0x080C20A4): shown, appearing (a sound), 5 ticks; then, on a solid panel, the stand, else the leave.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = ANIM_APPEAR
        me.timer = 5
        battle.play_sound(SOUND_APPEAR)
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    next_action(me, if field.solid(me.panel_x, me.panel_y) then 4 else 0xC)
end

-- Action 4 (0x080C20E2): standing, 20 ticks; then the throws.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_STAND
        me.timer = 0x14
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    next_action(me, 8)
end

-- Action 8 (0x080C2108), by its phase: 9 ticks, the balls counted (his first parameter and three); a ball every 24
-- ticks (the throw's animation restarting with each); then 31 ticks standing, and the leave.
local function throws(me: Object)
    local phase = me.phase
    if phase == 0 then
        -- 0x080C2128.
        if me.phase_init == 0 then
            me.timer = 8
            me.timer2 = (me.state :: State).level + 3
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = 4, 0
        end
    elseif phase == 4 then
        -- 0x080C214C.
        if me.phase_init == 0 then
            me.anim, me.anim_loaded = ANIM_THROW, 0xFF
            me.timer = 0x18
            ball.spawn(me, me.panel_x + me.facing, me.panel_y, BALL_Z, BALL_WAIT)
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t >= 0 then
            return
        end
        me.phase_init = 0
        local left = me.timer2 - 1
        me.timer2 = left
        if left <= 0 then
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        -- 0x080C2198.
        if me.phase_init == 0 then
            me.timer = 0x1E
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.anim = ANIM_STAND
            next_action(me, 0xC)
        end
    else
        error(string.format("NumbrBl's NumberMan: phase %#x of action 8 reads past its table (0x080C211C)", phase))
    end
end

-- Action 0xC (0x080C21BE): leaving, 5 ticks; then his controller goes on, and he goes. (He would also end the object
-- at his +0x60, which nothing sets: the game's write through a null pointer lands nowhere.)
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_LEAVE
        me.timer = 5
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local c = (me.state :: State).controller
    if c then
        local cs = c.state :: any
        cs.busy = false
    end
    -- (0x080C21E8: the state byte alone; his action and phase stay.)
    me.lifecycle = "destroy"
end

local ACTIONS: { [number]: (Object) -> () } = { [0] = appear, [4] = stand, [8] = throws, [0xC] = leave }

-- 0x080C203C: his sprite (with its shadow) and his palette by the level, facing his side's way; on his panel, on the
-- ground. (Its animation byte is left 0xFF, the sprite's animation with it; his first action sets it.)
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0xFF
    sprite.palette = PALETTE_ROWS[(me.state :: State).level] or 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_lifecycle("update")
end

numberman.kind = define.kind {
    id = "numbrbl/numberman",
    pool = "actor",
    state = { controller = "object", level = "u8" },
    -- 0x080C2018: by state, then the sprite (stepping while dimmed).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local act = ACTIONS[me.action]
            if not act then
                error(string.format("NumbrBl's NumberMan: action %#x reads past its table (0x080C2094)", me.action))
            end
            act(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080C21EC: NumberMan on (x, y) for `controller` (his side and facing its), with its element and the damage word
-- `damage`, his level (and the balls less three) `level`; he holds the controller's `busy` and sets it.
function numberman.spawn(controller: Object, x: number, y: number, damage: number, level: number): Object?
    local o = battle.spawn(numberman.kind, Vec3.new(y, controller.element, controller.alliance))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = controller.element
        o.related1 = controller
        o.alliance, o.flip = controller.alliance, controller.flip
        o:set_damage_word(damage)
        local s = o.state :: State
        s.controller, s.level = controller, level
        local cs = controller.state :: any
        cs.busy = true
    end
    return o
end

return numberman
