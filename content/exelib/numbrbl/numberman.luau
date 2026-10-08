--!strict
-- NumbrBl's NumberMan (actor object #0x45: EXE5's 0x080C2018, EXE4's 0x080C8CC4, the same code; EXE6's #0x45 draws
-- the user, its own), made for a game's look (`numberman.make`): the navi himself, on the panel the user warped out
-- of. He appears (a sound) and, on a solid panel, stands 21 ticks; then sets balls (@exelib/numbrbl/ball: the chip's
-- first parameter and three) down in front of him, 24 ticks apart, waits 31 ticks and leaves; off a solid panel he
-- leaves at once. Leaving, he lets his controller (@exelib/numbrbl/controller) go on. His sprite steps while dimmed.
--
-- The games part in his look: his sprite, palette, appearing sound and throwing animation, his balls, and his init
-- (EXE4's loads his standing animation, `loads_standing`). He throws toward the front (EXE5's 0x0800C6D2 by his
-- facing; EXE4's 0x0800AB88 by his side: the same, his flip 0, as EXE4's spawn copies his side alone).

local numberman = {}

export type State = {
    -- The controller waiting for him (EXE5's +0x54, EXE4's +0x50: its `busy`).
    controller: Object?,
    -- The balls he sets down (his chip's: the original's his level, Param0, and three).
    balls: number,
}

type ControllerState = { busy: boolean }

local ANIM_APPEAR, ANIM_STAND, ANIM_LEAVE = 3, 0, 4
-- The balls sit 16 pixels up and wait 10 ticks (plus one) before rolling.
local BALL_Z, BALL_WAIT = 0x10_0000, 0xA

local function next_action(me: Object, a: number)
    me.action, me.phase, me.phase_init = a, 0, 0
end

-- NumbrBl's NumberMan for `look`'s game: his kind and `spawn`.
function numberman.make(look: NumbrBlNumberManLook): NumbrBlNumberMan
    local ball = look.ball

    -- Action 0 (EXE5's 0x080C20A4): shown, appearing (a sound), 5 ticks; then, on a solid panel, the stand, else the
    -- leave.
    local function appear(me: Object)
        if me.phase_init == 0 then
            me.visible = true
            me.anim = ANIM_APPEAR
            me.timer = 5
            battle.play_sound(look.appear)
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t > 0 then
            return
        end
        next_action(me, if field.solid(me.panel_x, me.panel_y) then 4 else 0xC)
    end

    -- Action 4 (EXE5's 0x080C20E2): standing, 20 ticks; then the throws.
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

    -- Action 8 (EXE5's 0x080C2108), by its phase: 9 ticks, the balls counted (his chip's); a ball every 24 ticks (the
    -- throw's animation restarting with each); then 31 ticks standing, and the leave.
    local function throws(me: Object)
        local phase = me.phase
        if phase == 0 then
            -- EXE5's 0x080C2128.
            if me.phase_init == 0 then
                me.timer = 8
                me.timer2 = (me.state :: State).balls
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = t
            if t < 0 then
                me.phase, me.phase_init = 4, 0
            end
        elseif phase == 4 then
            -- EXE5's 0x080C214C.
            if me.phase_init == 0 then
                me.anim, me.anim_loaded = look.throw_anim, 0xFF
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
            -- EXE5's 0x080C2198.
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
            error(string.format("NumbrBl's NumberMan: phase %#x of action 8 reads past its table", phase))
        end
    end

    -- Action 0xC (EXE5's 0x080C21BE): leaving, 5 ticks; then his controller goes on, and he goes. (He would also end
    -- the object at his +0x60, which nothing sets: the game's write through a null pointer lands nowhere.)
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
            (c.state :: ControllerState).busy = false
        end
        -- (The state byte alone; his action and phase stay.)
        me.lifecycle = "destroy"
    end

    local ACTIONS: { [number]: (Object) -> () } = { [0] = appear, [4] = stand, [8] = throws, [0xC] = leave }

    -- EXE5's 0x080C203C: his sprite (with its shadow) and his palette, facing his side's way; on his panel, on the
    -- ground. EXE5's leaves his animation byte 0xFF, the sprite's animation with it; EXE4's (0x080C8CE8) sets his
    -- animation 0 and loads it, its copy 0xFF (`loads_standing`). His first action sets his own.
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "ground"
        if look.loads_standing then
            me.anim, me.anim_loaded = ANIM_STAND, 0xFF
            sprite:set_animation(ANIM_STAND)
        else
            me.anim = 0xFF
        end
        sprite.palette = look.palette
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me:set_lifecycle("update")
    end

    local kind = new.kind {
        id = look.id,
        pool = "actor",
        state = { controller = "object", balls = "u8" },
        -- EXE5's 0x080C2018: by state, then the sprite (stepping while dimmed).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                local act = ACTIONS[me.action]
                if not act then
                    error(string.format("NumbrBl's NumberMan: action %#x reads past its table", me.action))
                end
                act(me)
            else
                me:free()
                return
            end
            me:update_sprite_while_dimmed()
        end,
    }

    -- EXE5's 0x080C21EC: NumberMan on (x, y) for `controller` (his side and facing its), with its element and the
    -- damage word `damage`, setting `balls` balls down; he holds the controller's `busy` and sets it. (His position is
    -- the spawner's registers, which his init overwrites but for Z.)
    local function spawn(controller: Object, x: number, y: number, damage: number, balls: number): Object?
        local o = battle.spawn(kind, Vec3.new(y, controller.element, controller.alliance))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = controller.element
            o.related1 = controller
            o.alliance, o.flip = controller.alliance, controller.flip
            o:set_damage_word(damage)
            local s = o.state :: State
            s.controller, s.balls = controller, balls
            ;(controller.state :: ControllerState).busy = true
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return numberman
