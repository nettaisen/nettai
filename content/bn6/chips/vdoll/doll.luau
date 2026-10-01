--!strict
-- VDoll's doll (the original's attack object #0x7A, `sub_80D46B8`): a field
-- object with 50 HP thrown three panels ahead in a 60-tick arc, which can
-- hit in its last five ticks. Where it lands the panel turns to poison.
-- The first damage it takes after landing curses the other side
-- (curse.luau): a dimming that hits every combatant there with that
-- damage. See docs/engine/shot-chips.md §14.2.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local curse = require("./curse")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local doll = {}

export type State = {
    -- The panel it aims at (Param3, Param4).
    target_x: number,
    target_y: number,
    -- Hurt (ExtraVars), the damage it took (ExtraVars+4), and still flying
    -- (ExtraVars+0xC).
    hit: boolean,
    damage_taken: number,
    flying: boolean,
}

local SPRITE = asset.sprite("voodoo-doll")
-- Its identity: a field object, with the look something that takes its
-- place shows (DustMan's junk).
local IDENTITY = define.identity {
    id = "vdoll",
    class = "field_object",
    version = 3,
    ai_index = 1,
    look = { sprite = asset.sprite("voodoo-doll"), anim = 1, palette = 0, shadow = true },
}
local HP, LIFETIME = 0x32, 0x1E0
-- Its arc: 60 ticks to the target's center, falling 0x2000 a tick; it can
-- hit in its last five.
local FLIGHT_TICKS, GRAVITY, HITS_FROM = 0x3C, -0x2000, 5
local HIT_MOD, SET_HIT_MOD = 1, 3
local SOUND_POISON = asset.sound("panel-poison")
-- The curse's damage word: the damage taken, hit parameter 0xB2.
local CURSE_HIT_PARAM = 0xB2_0000
-- A puff 16 pixels up; the curse's own puff 12 up.
local PUFF_UP, CURSE_PUFF_UP = 0x10_0000, 0xC_0000
-- Its kind among absorbed obstacles.
-- How it looks absorbed (the original's obstacle kind's sprite: its own).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }

-- The actions (the object's action byte), and the destroyed action's
-- phases.
local DESTROYED, SET = 2, 8
local START, LEAVING, CURSING = 0x0, 0x4, 0x8

-- (The HUD element it turns on with the curse and off as it goes,
-- `sub_801DACC(0x40)` and `sub_801DA48(0x40)`, is drawn only, and so is
-- `sprite_clearFinalPalette`.)

-- `sub_80D47C0` (action 0): not while dimmed. The arc to the target; in
-- its last five ticks it can hit; on the 61st it lands: on a solid panel,
-- which turns to poison, or broken in a hole.
local function fly(me: Object)
    local s = me.state :: State
    if battle.dimmed() then
        return
    end
    if me.phase_init == 0 then
        local cx, cy = field.center(s.target_x, s.target_y)
        me.vel = trajectory.in_ticks(me.pos, cx, cy, 0, FLIGHT_TICKS, GRAVITY)
        me.timer2 = FLIGHT_TICKS
        s.flying = true
        me.phase_init = 4
    end
    if me.timer2 < HITS_FROM then
        me.collision:set_region(regions.single)
    end
    local left = me.timer2 - 1
    me.timer2 = bit32.band(left, 0xFFFF)
    if left >= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        return
    end
    if not field.solid(me.panel_x, me.panel_y) then
        me.action = DESTROYED
        me.phase, me.phase_init = 0, 0
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:update_collision_panels()
    field.set_type(me.panel_x, me.panel_y, "poison")
    battle.play_sound(SOUND_POISON)
    s.flying = false
    me.action = SET
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D4870` (action 8): set down, an obstacle of no side that anything
-- can hit.
local function set_down(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me:reset_collision_types(collision.neutral_body, collision.everything, SET_HIT_MOD)
    end
end

-- Gone quietly: out of the field's bookkeeping, and freed at the next
-- update.
local function go(me: Object)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- A puff 16 pixels up, then gone.
local function puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + PUFF_UP)), effects.puff)
    go(me)
end

-- `sub_80D48AE`: how it leaves: blinking out; carried off; a chip's
-- removal (absorbed, or a puff); or, hurt after landing, the curse.
local function leave(me: Object)
    local s = me.state :: State
    local blink = obstacle.blink_out(me)
    if blink == "blinking" then
        return
    elseif blink == "done" or me:status("carried") then
        go(me)
        return
    end
    local how = obstacle.removal(me)
    if how ~= "broken" then
        if how == "absorbed" then
            obstacle.fly_to_absorber(me, ABSORBED)
            go(me)
        else
            puff(me)
        end
        return
    end
    if s.damage_taken == 0 then
        puff(me)
        return
    end
    if battle.dimmed() then
        return
    end
    if not s.hit or battle.over() then
        puff(me)
        return
    end
    if s.flying then
        go(me)
        return
    end
    -- The curse.
    me.phase, me.phase_init = CURSING, 0
    me.visible = false
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + CURSE_PUFF_UP)), effects.puff)
    local controller = curse.spawn(me, int.u32(CURSE_HIT_PARAM + s.damage_taken))
    dimming.start(me.alliance, true, controller, me, curse.telop)
    obstacle.release_tracking(me)
end

-- `sub_80D49A0`: hidden; once the dimming is over, gone.
local function cursing(me: Object)
    me.visible = false
    if battle.dimmed() then
        return
    end
    obstacle.unregister(me)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_lifecycle("destroy")
end

-- `sub_80D4888` (action 2): by phase.
local function destroyed(me: Object)
    local phase = me.phase
    if phase == START then
        me.phase, me.phase_init = LEAVING, 0
    elseif phase == LEAVING then
        leave(me)
    elseif phase == CURSING then
        cursing(me)
    else
        error(string.format("VDoll destroyed phase %#x reads past JumpTable80D489C", phase))
    end
end

-- Its action table (`off_80D479C`): its own actions and the framework's
-- shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = fly,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = set_down,
}

-- `sub_80D4754`: any damage it takes this tick breaks it, and is kept for
-- the curse.
local function tick(me: Object)
    local s = me.state :: State
    me.visible = true
    obstacle.take_hits(me, "keeps_damage")
    me:hit_spark()
    obstacle.tick_lifetime(me)
    local c = me.collision
    local taken = c.final_damage
    if taken ~= 0 then
        me.hp = 0
        s.hit = true
        s.damage_taken = taken
        c:set_region(nil)
    end
    local a = obstacle.react(me, "breaks")
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("VDoll action %d reads past off_80D479C", a))
        end
        action(me)
    end
    me:load_or_step_sprite()
    c:present()
end

-- `sub_80D46D8`: shown in its second animation, with 50 HP, an object of
-- its side that the other side's hits reach.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 1, 0
    sprite:set_animation(1)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_panel_from_coordinates()
    me.identity = IDENTITY
    me.hp, me.max_hp = HP, HP
    me.element = 0
    me.timer = LIFETIME
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_object, collision.hit_by_other_side_or_blockers, HIT_MOD)
    me.collision:set_region(nil)
    me.collision:set_hit_effect(sparks.plain)
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

doll.kind = define.kind {
    id = "vdoll/doll",
    pool = "attack",
    state = {
        target_x = "u8",
        target_y = "u8",
        hit = "bool",
        damage_taken = "u32",
        flying = "bool",
    },
    -- `sub_80D46B8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80D49F6`: a thrower for `throw.action` that throws a doll at the
-- panel three ahead of the user (the original adds the column into a word
-- over the row, so a column below 0 would borrow from it). It runs while
-- dimmed and is one of its side's field objects (`sub_80D49CC`).
function doll.thrower(): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local word = int.u32(user.panel_y * 0x100_0000 + (user.panel_x + 3 * user.facing) * 0x1_0000)
            local o = battle.spawn(doll.kind, pos)
            if o then
                local s = o.state :: State
                s.target_x = bit32.band(bit32.rshift(word, 16), 0xFF)
                s.target_y = bit32.rshift(word, 24)
                o:set_damage_word(damage)
                o.alliance, o.flip = user.alliance, user.flip
                o.run_while_dimmed = true
                obstacle.register(o, user.alliance, 1)
            end
            return o
        end,
    }
end

return doll
