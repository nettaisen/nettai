--!strict
-- EXE5's Skullys (action 0x55, 0x080EFC30, EXE5's own): the navi calls up a skull (attack object #0x88, 0x080D7290,
-- EXE5's own; EXE6 numbers another object so) on the panel ahead, 20 pixels up; it confuses what it hits (status
-- 0x20, 480 ticks). Each skull lasts 30 ticks and, 15 ticks in, calls the next on the panel beside it toward the
-- nearest enemy body in a line (ahead, up or down, ahead first on a tie), three in all. The three chips differ in
-- the skulls' palette.

local collision = require("../../rules/collision")
local status = require("../../rules/status")

local skully = {}

-- A chip's skulls: their palette (Param1 by 2), how many (Param2), the ticks before each calls the next (Param3).
export type Skulls = { palette: number, count: number, every: number }

export type State = {
    skulls: Skulls,
    -- Skulls still to come after this one (Param2).
    left: number,
}

local SPRITE = asset.sprite("skull")
local LIFETIME = 30
local HEIGHT = 0x14_0000
-- The enemy bodies by side (0x080D73D4).
local ENEMY_BODIES = { [0] = 0x04000000, [1] = 0x08000000 }
-- No enemy in that line.
local NONE = 0xFF

-- 0x080D73DC: the steps from (x, y) by (dx, dy) to the first enemy body, or NONE off the field.
local function steps_to_enemy(x: number, y: number, dx: number, dy: number, enemies: number): number
    local n = 0
    while true do
        n += 1
        x += dx
        y += dy
        if not field.valid(x, y) then
            return NONE
        end
        if bit32.btest(field.flags(x, y), enemies) then
            return n
        end
    end
end

-- 0x080D7356: the panel beside it toward the nearest enemy body in a line: ahead, up or down.
local function next_panel(me: Object): (number, number)
    local enemies = ENEMY_BODIES[me.alliance]
    local x, y = me.panel_x, me.panel_y
    local dx, dy = me.facing, 0
    local best = steps_to_enemy(x, y, dx, dy, enemies)
    for _, d in { -1, 1 } do
        local n = steps_to_enemy(x, y, 0, d, enemies)
        if n < best then
            best, dx, dy = n, 0, d
        end
    end
    return x + dx, y + dy
end

local spawn: (owner: Object, x: number, y: number, element: number, damage: number, z: number, skulls: Skulls, left: number) -> Object?

-- 0x080D72F8: its hits take its region; it lasts 30 ticks, calling the next skull on its 15th.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    me.timer2 -= 1
    if me.timer2 == 0 then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local s = me.state :: State
    if s.left ~= 0 then
        local t = int.u16(me.timer - 1)
        me.timer = t
        if t == 0 then
            local x, y = next_panel(me)
            spawn(me, x, y, me.element, me.damage + me.stamina * 0x10000, me.pos.z, s.skulls, s.left)
        end
    end
    c:present()
end

-- 0x080D72B4.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local s = me.state :: State
    -- sub_8011504: its sprite (with a shadow), its collision.
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        -- (The game goes on without collision data, writing through a null pointer.)
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, 3)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    sprite.palette = s.skulls.palette * 2
    me.collision.status_base = status.confuse_480
    me.collision:present()
    s.left = int.u8(s.left - 1)
    me.timer = s.skulls.every
    me.timer2 = LIFETIME
    me:set_lifecycle("update")
    tick(me)
end

skully.kind = define.kind {
    id = "skully/skull",
    pool = "attack",
    state = { skulls = "record:skulls", left = "u8" },
    -- 0x080D7290: by its lifecycle, then a sprite step.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080D7340: a skull on (x, y) at height `z` with `element` and the damage word, `left` more to come.
spawn = function(owner: Object, x: number, y: number, element: number, damage: number, z: number, skulls: Skulls, left: number): Object?
    local o = battle.spawn(skully.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(int.u32(damage))
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        local s = o.state :: State
        s.skulls, s.left = skulls, left
    end
    return o
end

function skully.skulls(palette: number): Skulls
    return define.record("skulls", { palette = palette, count = 3, every = 0x0F })
end

export type ActionState = {
    -- Ticks into the call, then of the recovery (AIAttackVars+0x10).
    timer: number,
}

local CALL, RECOVER = 0x0, 0x4
local SOUND = asset.sound("boomerang")

-- A chip's Skully: its skulls.
function skully.action(id: string, skulls: Skulls): Action
    -- 0x080EFC4C: the call; the skull on the 5th tick, 16 in all.
    local function call(me: Object, s: ActionState)
        if me.step_init == 0 then
            me:set_animation(12)
            me:set_status("using_action", true)
            battle.play_sound(SOUND)
            me.step_init = 4
            s.timer = 0
        end
        if s.timer == 4 then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            spawn(me, me.panel_x + me.facing, me.panel_y, 0, damage, HEIGHT, skulls, skulls.count)
        end
        local t = s.timer + 1
        s.timer = t
        if t > 0x0F then
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- 0x080EFCA2: 6 ticks, then back to idle.
    local function recover(me: Object, s: ActionState)
        if me.step_init == 0 then
            s.timer = 5
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        me:set_status("using_action", false)
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end

    return define.action {
        id = id,
        state = { timer = "u16" },
        args = skulls,
        update = function(me: Object, s: ActionState)
            local step = me.step
            if step == CALL then
                call(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("Skully phase %#x reads past its phase table", step))
            end
        end,
    }
end

return skully
