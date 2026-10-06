--!strict
-- EXE5's CactBal1-3 (action 0x48, 0x080EF0F4) and their cactus ball (attack object #0x5E, 0x080D1A88): EXE5's own
-- (EXE6's BblStar's action, `sub_80ED2F8`, is the nearest code; EXE6's attack object #0x5E is another thing). The navi
-- raises its buster arm and, on the action's second tick, throws a wooden ball from where it stands, 16 pixels up.
-- The ball hops two panels ahead in 15 ticks, then one panel at a time in 10, as long as it lands on solid panels and
-- stays on the field; hitting something, it hops a panel back and goes on, until its fifth hop, after which a hit ends
-- it. It hits navis (hit modifier 1) wherever it moves onto a new panel. The three differ in the ball's palette (the
-- chip's first parameter, times 2) and damage.

local arm = require("../../lib/arm")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local cactbal = {}

-- The ball's look: sheet 10-30, animation 4, no shadow (0x080D1AB2, `sub_8011504`'s word 0x00041030).
local SPRITE = asset.sprite("cactus")
local ANIM = 4
local THROW_SOUND, LAND_SOUND = asset.sound("cactus-throw"), asset.sound("cactus-hop")
-- Its collision (0x04050100): an attack that hits navis, hit modifier 1.
local HIT_MOD = 1
-- Falling 0.25 pixels a tick faster each tick.
local GRAVITY = int.i32(0xFFFFC000)
-- The first hop: two panels in 15 ticks; then one panel in 10.
local FIRST_HOPS, FIRST_TICKS, HOP_TICKS = 2, 15, 10
-- Hits end it from its fifth hop on.
local HOPS_BEFORE_ENDING = 4

-- Its actions.
local FIRST, HOP, BACK = 0, 4, 8

export type State = {
    -- Hops after the first (+0x60).
    hops: number,
    -- Its palette parameter (its first parameter: the chip's).
    palette: number,
}

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- A hop's start: `panels` panels ahead (or back), in `ticks` ticks, toward the panel's center on the ground.
local function hop_to(me: Object, panels: number, ticks: number)
    local x, y = me.panel_x + panels * me.facing, me.panel_y
    me.future_panel_x, me.future_panel_y = x, y
    local cx, cy = field.center(x, y)
    me.vel = trajectory.in_ticks(me.pos, cx, cy, 0, ticks, GRAVITY)
    me.timer = ticks
end

-- A hop's tick: on the last, it lands on its panel (the sound) and says so; else it moves, falling.
local function hopping(me: Object): boolean
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        battle.play_sound(LAND_SOUND)
        return true
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
    me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    return false
end

-- 0x080D1B58: the first hop; a hit ends it.
local function first(me: Object)
    if me.collision.hit_flags ~= 0 then
        finish(me)
        return
    end
    if me.phase_init == 0 then
        hop_to(me, FIRST_HOPS, FIRST_TICKS)
        me.phase_init = 4
        return
    end
    if hopping(me) then
        me:set_action(HOP)
    end
end

-- 0x080D1BFC: a hop ahead (counted); a hit sends it back a panel, or ends it from its fifth hop on.
local function hop(me: Object)
    local s = me.state :: State
    if me.collision.hit_flags ~= 0 then
        if s.hops >= HOPS_BEFORE_ENDING then
            finish(me)
        else
            me:set_action(BACK)
        end
        return
    end
    if me.phase_init == 0 then
        hop_to(me, 1, HOP_TICKS)
        s.hops += 1
        me.phase_init = 4
        return
    end
    if hopping(me) then
        me.phase, me.phase_init = 0, 0
    end
end

-- 0x080D1CB0: a hop back a panel; a hit ends it.
local function back(me: Object)
    if me.collision.hit_flags ~= 0 then
        finish(me)
        return
    end
    if me.phase_init == 0 then
        hop_to(me, -1, HOP_TICKS)
        me.phase_init = 4
        return
    end
    if hopping(me) then
        me:set_action(HOP)
    end
end

-- 0x080D1AEC: the hits against it resolve and spark (a hit takes it off its panels); its action; then, but in its
-- first hop, it ends off solid ground; it ends off the field (past 160 pixels either way); else, moved onto another
-- panel, it covers it.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local was_x, was_y = me.panel_x, me.panel_y
    local a = me.action
    if a == FIRST then
        first(me)
    elseif a == HOP then
        hop(me)
    elseif a == BACK then
        back(me)
    else
        error(string.format("the cactus ball's action %#x reads past 0x080D1B4C", a))
    end
    if me.action ~= FIRST and not field.solid(me.panel_x, me.panel_y) then
        finish(me)
    elseif int.u32(int.asr(me.pos.x, 16) + 160) > 320 then
        finish(me)
    elseif me.lifecycle ~= "destroy" and (me.panel_x ~= was_x or me.panel_y ~= was_y) then
        c:set_region(regions.single)
    end
    c:present()
end

-- 0x080D1AAC: on the panel under it, its look and collision, the spark of its element, its palette (the chip's first
-- parameter, times 2); the throw's sound; the first update at once.
local function init(me: Object)
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- (The setup's shadow byte is 0: `sprite_noShadow`, 0x0800F1C4, the first part drawn with the sprite.)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = ANIM, 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.collision:set_hit_effect(sparks.of_element(me.element))
    ;(me.state :: State).hops = 0
    sprite.palette = (me.state :: State).palette * 2
    battle.play_sound(THROW_SOUND)
    me.lifecycle = "update"
    tick(me)
end

cactbal.ball = new.kind {
    id = "cactbal/ball",
    pool = "attack",
    state = { hops = "u32", palette = "u8" },
    -- 0x080D1A88.
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

-- 0x080D1D54: a ball of `owner`'s at `pos`, wood, with the damage word and its palette parameter.
local function throw(owner: Object, pos: Vec3, damage: number, palette: number): Object?
    local o = battle.spawn(cactbal.ball, pos)
    if o then
        o.element = 4
        o.alliance, o.flip = owner.alliance, owner.flip
        o:set_damage_word(damage)
        ;(o.state :: State).palette = palette
    end
    return o
end

export type ActionState = {
    -- The phase's count (AIAttackVars+0x10).
    timer: number,
}

-- The phases, on the attack's step.
local THROW, RECOVER = 0x0, 0x4
local THROW_ANIM = 0x0E
local THROW_TICKS, THROW_AT, RECOVER_TICKS = 10, 9, 5

-- The action of a chip whose ball is in `palette` (the chip's first parameter).
function cactbal.action(palette: number): Action
    -- 0x080EF110: the arm raised; on the count's 9 the ball; then the recovery.
    local function throwing(me: Object, s: ActionState)
        if me.step_init == 0 then
            me:set_animation(THROW_ANIM)
            arm.raise(me)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = THROW_TICKS
            me.step_init = 4
        end
        if s.timer == THROW_AT then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            local p = me.pos
            throw(me, Vec3.new(p.x, p.y, p.z + 0x10_0000), damage, palette)
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- 0x080EF162: 6 ticks, then the arm goes and back to idle.
    local function recover(me: Object, s: ActionState)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    return new.action {
        state = { timer = "u16" },
        args = { palette = palette },
        -- 0x080EF0F4.
        update = function(me: Object, s: ActionState)
            local step = me.step
            if step == THROW then
                throwing(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("CactBal's phase %#x reads past 0x080EF100", step))
            end
        end,
    }
end

return cactbal
